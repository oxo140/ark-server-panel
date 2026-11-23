'use strict';
const $=s=>document.querySelector(s),$$=s=>[...document.querySelectorAll(s)];
let key=sessionStorage.getItem('ark-access')||'',snapshot=null,selected=null,configHash=null,configName=null,configOriginal='',polling=false;
if(location.hash.length>1){key=location.hash.slice(1);sessionStorage.setItem('ark-access',key);history.replaceState(null,'',location.pathname);}
const escape=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
async function api(action,values={}){
  const response=await fetch('/api',{method:'POST',headers:{'Content-Type':'application/json','Authorization':`Bearer ${key}`},body:JSON.stringify({action,...values})});
  const data=await response.json();if(!response.ok){if(response.status===401){$('#shell').hidden=true;$('#login').hidden=false;}throw Error(data.error||'Requête refusée.');}return data;
}
let toastTimer;
function toast(message,error=false){const box=$('#toast');box.textContent=message;box.classList.toggle('error',error);box.hidden=false;clearTimeout(toastTimer);toastTimer=setTimeout(()=>box.hidden=true,error?15000:8000);}
async function run(button,fn){button.disabled=true;try{await fn();}catch(e){if($('#detail-dialog').open){$('#detail-message').textContent=e.message;$('#detail-message').classList.add('danger');}else toast(e.message,true);}finally{button.disabled=false;}}
function card(row){const p=row.profile,busy=row.job?.running||row.operation?.running;
  return `<article class="server-card"><div class="card-top"><span class="map-icon">◈</span><span class="status ${row.running?'running':''}">${row.running?'Processus actif':busy?'Opération en cours':'Arrêté'}</span></div><h3>${escape(p.name)}</h3><div class="card-map">${escape(p.map)}</div><div class="card-data"><div><span>PORT DE JEU</span><strong>${p.gamePort} · UDP</strong></div><div><span>PORT RCON</span><strong>${p.rconPort} · TCP</strong></div></div><div class="card-actions"><button class="${row.running?'':'primary'}" data-action="${row.running?'stop':'start'}" data-id="${p.id}" ${busy?'disabled':''}>${row.running?'Arrêter proprement':'Démarrer'}</button><button data-action="detail" data-id="${p.id}">Gérer</button><button data-action="install" data-id="${p.id}" ${row.running||busy?'disabled':''}>Installer / mettre à jour</button><button data-action="edit" data-id="${p.id}" ${row.running||busy?'disabled':''}>Modifier</button><button data-action="deleteProfile" data-id="${p.id}" ${row.running||busy?'disabled':''} title="Supprimer le profil, conserver les fichiers">Supprimer</button></div>${row.operation?`<p class="card-note ${row.operation.success===false?'danger':''}">${escape(row.operation.message)}</p>`:''}${row.job?.running?'<p class="card-note">SteamCMD travaille. Consultez les journaux.</p>':row.job?.success===false?`<p class="card-note danger">SteamCMD terminé avec une erreur (${row.job.exitCode}). Consultez les journaux.</p>`:''}</article>`;
}
function render(){const rows=snapshot.profiles;$('#total').textContent=rows.length;$('#online').textContent=rows.filter(r=>r.running).length;$('#tasks').textContent=rows.filter(r=>r.job?.running||r.operation?.running).length;$('#nav-count').textContent=rows.length;
  const html=rows.length?rows.map(card).join(''):'<div class="empty"><div><h3>Votre premier monde commence ici.</h3><p>Ajoutez un profil pour installer un serveur ARK ou gérer une installation existante sur cette machine.</p><button class="primary add-profile">+ Ajouter mon premier serveur</button></div></div>';
  $('#dashboard-servers').innerHTML=html;$('#server-list').innerHTML=html;
  $('#version').textContent=`v${snapshot.version}`;$('#footer-version').textContent=snapshot.version;$('#data-dir').textContent=snapshot.dataDirectory;
  if(!$('#settings-form').dataset.loaded){$('#settings-form').elements.repository.value=snapshot.settings.repository;$('#settings-form').elements.steamcmd.value=snapshot.settings.steamcmd;$('#settings-form').dataset.loaded='true';}
  const u=snapshot.updates;const short=s=>s?s.slice(0,12):'Non disponible';
  $('#update-status').innerHTML=`<div class="update-line"><span>Version installée</span><b>v${escape(snapshot.version)}</b></div><div class="update-line"><span>Commit installé</span><code>${escape(short(snapshot.revision))}</code></div><div class="update-line"><span>Dernière publication</span><b>${escape(u.latestVersion?'v'+u.latestVersion:'Pas encore vérifiée')}</b></div><div class="update-line"><span>Commit de main</span><code>${escape(short(u.branchSha))}</code></div><div class="update-line"><span>Dernière vérification</span><span>${u.checkedAt?new Date(u.checkedAt*1000).toLocaleString('fr-FR'):'En attente'}</span></div><p class="${u.error?'danger':'teal'} small">${escape(u.error|| (u.pending?'Mise à jour prête à démarrer.':u.available?'Une publication différente est disponible.':u.checkedAt?'Votre version correspond à la dernière publication.':'Vérification en cours…'))}</p>${u.branchSha&&u.latestSha&&u.branchSha!==u.latestSha?'<p class="muted small">main contient des commits différents de la dernière publication. Seules les versions publiées sont installables.</p>':''}${snapshot.lastUpdate?.message?`<p class="notice warning">${escape(snapshot.lastUpdate.message)}</p>`:''}`;
  $('#install-update').hidden=!u.available||u.pending;$('#install-update').disabled=snapshot.updateBusy;$('#restart-panel').hidden=!u.pending;
}
async function refresh(){if(polling)return;polling=true;try{snapshot=await api('status');$('#shell').hidden=false;$('#login').hidden=true;$('#connection-error').hidden=true;render();}catch(e){$('#connection-error').textContent=e.message||'Connexion interrompue. Nouvelle tentative automatique.';$('#connection-error').hidden=false;if(!snapshot)$('#login-error').textContent=e.message;}finally{polling=false;}}
function openProfile(p){const f=$('#profile-form');f.reset();f.elements.id.value=p?.id||'';if(p){for(const [k,v]of Object.entries(p))if(f.elements[k])f.elements[k].value=v;}else{f.elements.directory.value='C:\\ARK\\Serveur'+((snapshot?.profiles.length||0)+1);f.elements.gamePort.value=7777+(snapshot?.profiles.length||0)*10;f.elements.rconPort.value=27020+(snapshot?.profiles.length||0)*10;}$('#profile-title').textContent=p?'Modifier le serveur':'Ajouter un serveur';$('#profile-form .form-error').textContent='';$('#profile-dialog').showModal();}
function dirty(){return configName!==null&&$('#config-text').value!==configOriginal;}
function closeDialog(dialog){if(dialog.id==='detail-dialog'&&dirty()&&!confirm('Fermer sans enregistrer les modifications INI ?'))return;dialog.close();}
async function loadConfig(){if(dirty()&&!confirm('Recharger et abandonner les modifications non enregistrées ?'))return;const file=$('#config-file').value;const result=await api('readConfig',{id:selected.id,file});$('#config-text').value=result.text;configHash=result.hash;configName=file;configOriginal=result.text;}
async function logs(){const result=await api('logs',{id:selected.id});$('#logs').textContent=result.text;}
async function backups(){const result=await api('listBackups',{id:selected.id});$('#backup-list').innerHTML=result.backups.length?result.backups.map(name=>`<div class="backup-row"><span class="mono">${escape(name)}</span><button data-restore="${escape(name)}">Restaurer</button></div>`).join(''):'<p class="muted">Aucune sauvegarde terminée pour ce profil.</p>';}
document.addEventListener('click',e=>{const b=e.target.closest('button');if(!b)return;
  if(b.classList.contains('add-profile'))openProfile();
  if(b.classList.contains('close-dialog'))closeDialog(b.closest('dialog'));
  if(b.dataset.view){$$('.view').forEach(v=>v.hidden=v.id!=='view-'+b.dataset.view);$$('.nav').forEach(v=>v.classList.toggle('active',v===b));$('#breadcrumb').textContent=b.querySelector('span').textContent;}
  if(b.dataset.tab){$$('.tabs button').forEach(t=>t.classList.toggle('active',t===b));['logs','config','backups'].forEach(t=>$('#tab-'+t).hidden=t!==b.dataset.tab);run(b,async()=>{if(b.dataset.tab==='backups')await backups();});}
  if(b.dataset.restore)run(b,async()=>{if(!confirm('Restaurer cette sauvegarde ? Le serveur doit être arrêté. L’ancien dossier Saved sera conservé.'))return;const result=await api('restore',{id:selected.id,backup:b.dataset.restore});$('#detail-message').textContent=result.message;await refresh();});
  if(b.dataset.action)run(b,async()=>{
    const row=snapshot.profiles.find(r=>r.profile.id===b.dataset.id);if(!row)return;const p=row.profile;const action=b.dataset.action;
    if(action==='edit'){openProfile(p);return;}
    if(action==='detail'){selected=p;configName=null;configHash=null;configOriginal='';$('#config-text').value='';$('#detail-title').textContent=p.name;$('#detail-message').textContent='';$('#detail-message').classList.remove('danger');$('#rcon-output').hidden=true;$('#detail-dialog').showModal();$$('.tabs button')[0].click();await logs();return;}
    if(['stop','install','deleteProfile'].includes(action)&&!confirm({stop:`Sauvegarder et demander l’arrêt de ${p.name} via RCON ?`,install:`Lancer SteamCMD pour installer ou vérifier ${p.name} ? Les fichiers du jeu pourront être remplacés.`,deleteProfile:`Supprimer le profil ${p.name} ? Ses fichiers resteront sur le disque.`}[action]))return;
    const result=await api(action,{id:p.id});toast(result.message);await refresh();
  });
});
$$('dialog').forEach(d=>d.addEventListener('cancel',e=>{if(d.id==='detail-dialog'&&dirty()&&!confirm('Abandonner les modifications INI ?'))e.preventDefault();}));
$('#profile-form').addEventListener('submit',async e=>{e.preventDefault();const b=e.submitter;await run(b,async()=>{const profile=Object.fromEntries(new FormData(e.target));profile.gamePort=Number(profile.gamePort);profile.rconPort=Number(profile.rconPort);try{const result=await api('saveProfile',{profile});$('#profile-dialog').close();toast(result.message);await refresh();}catch(error){$('#profile-form .form-error').textContent=error.message;}});});
$('#settings-form').addEventListener('submit',e=>{e.preventDefault();run(e.submitter,async()=>{const result=await api('saveSettings',Object.fromEntries(new FormData(e.target)));toast(result.message);await refresh();});});
$('#login-form').addEventListener('submit',e=>{e.preventDefault();key=$('#access-key').value.trim();sessionStorage.setItem('ark-access',key);refresh();});
$('#logout').addEventListener('click',()=>{key='';sessionStorage.removeItem('ark-access');$('#shell').hidden=true;$('#login').hidden=false;});
$('#refresh').addEventListener('click',()=>refresh());
$('#check-updates').addEventListener('click',e=>run(e.currentTarget,async()=>{try{await api('checkUpdates');toast('Dépôt et dernière publication vérifiés.');}finally{await refresh();}}));
$('#install-update').addEventListener('click',e=>run(e.currentTarget,async()=>{if(!confirm('Télécharger et vérifier la dernière publication du dépôt configuré ?'))return;toast('Téléchargement de la version en cours…');const r=await api('installUpdate');toast(r.message);await refresh();}));
$('#restart-panel').addEventListener('click',e=>run(e.currentTarget,async()=>{const r=await api('restartPanel');toast(r.message);}));
$('#reload-logs').addEventListener('click',e=>run(e.currentTarget,logs));
$('#load-config').addEventListener('click',e=>run(e.currentTarget,loadConfig));
$('#save-config').addEventListener('click',e=>run(e.currentTarget,async()=>{if(configName!==$('#config-file').value||configHash===null)throw Error('Chargez ce fichier avant de l’enregistrer.');const r=await api('saveConfig',{id:selected.id,file:configName,text:$('#config-text').value,hash:configHash});configOriginal=$('#config-text').value;await loadConfig();$('#detail-message').textContent=r.message;}));
$('#rcon-form').addEventListener('submit',e=>{e.preventDefault();run(e.submitter,async()=>{const command=e.target.elements.command.value;if(/^(DoExit|Destroy|Wipe|Ban|Kick)/i.test(command)&&!confirm('Exécuter cette commande RCON : '+command+' ?'))return;const r=await api('rcon',{id:selected.id,command});$('#rcon-output').hidden=false;$('#rcon-output').textContent=r.message;});});
$('#create-backup').addEventListener('click',e=>run(e.currentTarget,async()=>{const r=await api('backup',{id:selected.id});$('#detail-message').textContent=r.message;await refresh();}));
$('#reload-backups').addEventListener('click',e=>run(e.currentTarget,backups));
window.addEventListener('beforeunload',e=>{if(dirty())e.preventDefault();});
$('#connection-label').textContent=location.hostname.startsWith('100.')?'Réseau Tailscale':'Connexion locale';
if(key)refresh();else $('#login').hidden=false;
setInterval(()=>{if(key)refresh();},5000);
