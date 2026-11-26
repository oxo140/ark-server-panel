'use strict';
(()=>{
  const colors=['#83e1b4','#86cff2'];
  const definitions=[
    {id:'cpu',name:'CPU',keys:['cpuPercent'],labels:['Utilisation'],unit:'%',scale:100},
    {id:'ram',name:'RAM',keys:['memoryUsedBytes'],labels:['Utilisée'],unit:'Gio',factor:1024**3},
    {id:'disk',name:'Disque',keys:['diskReadBytesPerSec','diskWrittenBytesPerSec'],labels:['Lecture','Écriture'],unit:'Mio/s',factor:1024**2},
    {id:'network',name:'Réseau',keys:['receivedBytesPerSec','transmittedBytesPerSec'],labels:['Réception','Émission'],unit:'Mio/s',factor:1024**2}
  ];
  let samples=[],latest=0;
  const fmt=n=>n.toLocaleString((window.ArkI18n?.locale()||'fr-FR'),{maximumFractionDigits:2});
  const unit=s=>window.ArkI18n?.translate(s)||s;
  const clock=t=>new Date(t*1000).toLocaleTimeString((window.ArkI18n?.locale()||'fr-FR'));
  function inspect(figure,index){
    const def=definitions.find(d=>d.id===figure.dataset.chart);
    if(!samples.length)return;
    index=Math.max(0,Math.min(samples.length-1,index));figure.dataset.point=index;
    const point=samples[index];
    figure.querySelector('.chart-reading').textContent=clock(point.at)+' · '+def.keys.map((k,i)=>def.labels[i]+' '+fmt((Number(point[k])||0)/(def.factor||1))+' '+unit(def.unit)).join(' · ');
  }
  window.renderHistory=m=>{
    const root=document.querySelector('#history-charts');if(!root)return;
    samples=(m.history||[]).filter(p=>Number.isFinite(p.at)&&p.at>=m.sampledAt-600&&p.at<=m.sampledAt);latest=m.sampledAt;
    document.querySelector('#history-status').textContent=m.historyPersistenceError?'Enregistrement indisponible : '+m.historyPersistenceError:samples.length?`${samples.length} mesures · ${clock(samples[0].at)} à ${clock(latest)} · sauvegarde locale toutes les 10 s`:'En attente des premières mesures…';
    if(!root.children.length)root.innerHTML=definitions.map(d=>`<figure class="history-chart" data-chart="${d.id}" tabindex="0" aria-label="Historique ${d.name} sur dix minutes. Flèches gauche et droite pour consulter les mesures."><figcaption>${d.name} <small>${unit(d.unit)}</small></figcaption><svg viewBox="0 0 600 170" role="img" aria-label="Courbe ${d.name}"></svg><p class="chart-reading">En attente…</p><div class="chart-legend">${d.labels.map((label,i)=>`<span style="--series-color:${colors[i]}">${label}</span>`).join('')}</div></figure>`).join('');
    for(const def of definitions){
      const figure=root.querySelector('[data-chart='+def.id+']');
      const max=def.scale||Math.max(def.id==='ram'?(m.memory?.totalBytes||0)/(def.factor||1):0,...samples.flatMap(p=>def.keys.map(k=>(Number(p[k])||0)/(def.factor||1))),0.1)*1.05;
      const paths=def.keys.map((key,i)=>{
        let last=0;const commands=samples.map(p=>{const n=Number(p[key]);if(!Number.isFinite(n))return '';const x=48+Math.max(0,Math.min(600,p.at-(latest-600)))/600*540,y=135-Math.min(max,Math.max(0,n/(def.factor||1)))/max*115;const command=!last||p.at-last>6?'M':'L';last=p.at;return command+x.toFixed(2)+' '+y.toFixed(2);}).join(' ');
        return `<path d="${commands}" stroke="${colors[i]}" stroke-width="2.5" fill="none"/>`;
      }).join('');
      figure.querySelector('svg').innerHTML=`<path d="M48 20H588 M48 77H588 M48 135H588" stroke="#334451" fill="none"/><text x="2" y="24">${fmt(max)}</text><text x="18" y="139">0</text><text x="48" y="160">−10 min</text><text x="530" y="160">Maintenant</text>${paths}`;
      if(document.activeElement!==figure)inspect(figure,samples.length-1);
    }
  };
  document.addEventListener('pointermove',e=>{const figure=e.target.closest('.history-chart');if(!figure||!samples.length)return;const rect=figure.querySelector('svg').getBoundingClientRect();const target=latest-600+Math.max(0,Math.min(1,((e.clientX-rect.left)/rect.width*600-48)/540))*600;let best=0;for(let i=1;i<samples.length;i++)if(Math.abs(samples[i].at-target)<Math.abs(samples[best].at-target))best=i;inspect(figure,best);});
  document.addEventListener('keydown',e=>{const figure=e.target.closest('.history-chart');if(!figure||!['ArrowLeft','ArrowRight','Home','End'].includes(e.key))return;e.preventDefault();const at=Number(figure.dataset.point)||0;inspect(figure,e.key==='Home'?0:e.key==='End'?samples.length-1:at+(e.key==='ArrowLeft'?-1:1));});
})();
