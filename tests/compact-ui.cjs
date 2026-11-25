const fs = require('node:fs');
const assert = require('node:assert/strict');
const {JSDOM} = require('../.tools/dom/node_modules/jsdom');
const dom = new JSDOM(fs.readFileSync('web/index.html', 'utf8'), {url:'http://localhost/', runScripts:'outside-only'});
const w = dom.window, d = w.document;
w.confirm = () => true;
for (const dialog of d.querySelectorAll('dialog')) {
  dialog.showModal = function () { this.open = true; };
  dialog.close = function () { this.open = false; };
}
const calls = [];
w.fetch = async (_url, options) => {
  const input = JSON.parse(options.body); calls.push(input);
  const data = input.action === 'serverStats' ? {running:true,ready:true,pid:123,memoryBytes:1024,uptimeSeconds:60,players:'No Players Connected',health:{state:'ready',rconReady:false,gamePortBound:true,advertised:true,message:'Serveur opérationnel localement'}}
    : input.action === 'readTuning' ? {fields:[],presets:[],hashes:{}}
    : input.action === 'readSchedule' ? {backupEnabled:true,backupHours:2,restartEnabled:false,restartTime:'06:00',updateEnabled:true,updateTime:'05:00',warningMinutes:5,message:'Maintenance {minutes}',lastResult:'Test OK'}
    : {text:'Test logs',message:'Enregistré'};
  return {ok:true,json:async()=>data};
};
async function settle() { for(let i=0;i<8;i++) await new Promise(r=>setImmediate(r)); }
(async () => {
  try {
    w.eval(fs.readFileSync('web/app.js','utf8') + `\ncatalog={maps:[]};snapshot={version:'test',revision:'test',settings:{repository:'test/repo',steamcmd:''},updates:{},profiles:[
      {profile:{id:'one',name:'Alpha <script>',map:'TheIsland_WP',gamePort:7777,rconPort:27020},running:true,health:{state:'ready'}},
      {profile:{id:'two',name:'Bravo',map:'TheIsland_WP',gamePort:7787,rconPort:27030},running:false},
      {profile:{id:'three',name:'Charlie',map:'TheIsland_WP',gamePort:7797,rconPort:27040},running:false,job:{running:true},progress:{percent:20}},
      {profile:{id:'four',name:'Delta',map:'TheIsland_WP',gamePort:7807,rconPort:27050},running:true,health:{state:'starting'}}]};render();`);
    assert.equal(d.querySelectorAll('#dashboard-servers .server-card').length,4);
    assert.equal(d.querySelectorAll('#running-servers .sidebar-server').length,2);
    assert.equal(d.querySelectorAll('#dashboard-servers .server-card script').length,0);
    for(const color of ['green','red','blue','orange']) assert.ok(d.querySelector('#dashboard-servers .state-'+color));
    assert.ok(d.querySelector('#dashboard-servers [data-action=join][data-id=two]').disabled);
    assert.equal(d.querySelector('#dashboard-servers [data-action=detail]').textContent,'Stats du serveur');
    d.querySelector('#dashboard-servers [data-action=testConnection]').click(); await settle();
    assert.ok(!d.querySelector('#detail-dialog').open);
    assert.ok(d.querySelector('#dashboard-servers .state-green').textContent.includes('jeu prêt'));
    d.querySelector('#dashboard-servers [data-action=detail]').click(); await settle();
    assert.ok(!d.querySelector('#tab-config').hidden);
    assert.ok(calls.some(c=>c.action==='readTuning'));
    d.querySelector('[data-tab=stats]').click(); await settle();
    assert.ok(d.querySelector('#server-stats').textContent.includes('123'));
    d.querySelector('[data-tab=schedule]').click(); await settle();
    assert.ok(d.querySelector('#schedule-form').elements.backupEnabled.checked);
    assert.equal(d.querySelector('#schedule-form').elements.backupHours.value,'2');
    d.querySelector('#schedule-form button').click(); await settle();
    const saved=calls.find(c=>c.action==='saveSchedule');
    assert.equal(saved.schedule.backupHours,2);assert.equal(saved.schedule.updateEnabled,true);
    d.querySelector('[data-tab=config]').click(); await settle();
    assert.ok(d.querySelector('#import-ini'));assert.ok(d.querySelector('#import-tuning'));assert.ok(d.querySelector('#export-tuning'));
    console.log('PASS: cards, escaping, four status colors, running-server sidebar, disabled offline join, server statistics, schedule loading and submission, import/export controls.');
  } finally { w.close(); }
})().catch(e=>{console.error(e);process.exitCode=1;});
