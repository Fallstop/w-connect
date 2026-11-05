// Install test-only dependencies: npm --prefix web ci
// Run: npm --prefix web test
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const {JSDOM} = require('jsdom');
const html = fs.readFileSync(path.join(__dirname,'index.html'),'utf8');
const script = fs.readFileSync(path.join(__dirname,'app.js'),'utf8');
const fresh = () => ({version:'0.1.0',platform:'linux',demo:false,emergency:false,controller:{hardware_tested:true,max_fans_per_bank:4,connected:true,name:'UNI FAN SL v1.8',vid:'0cf2',pid:'a100',error:null,lighting_supported:true},sensors:{cpu:43,gpu:39,source:'hwmon'},channels:Array.from({length:4},(_,i)=>({index:i,name:`Bank ${i+1}`,rpm:900+i*10,duty:45,target:45,mode:'curve',status:'Applied 45%'})),config:{controller:{allow_untested:false,serial:null},poll_ms:2000,channels:Array.from({length:4},(_,i)=>({name:`Bank ${i+1}`,fan_count:3,mode:'curve',source:'max',fixed_duty:50,curve:[{temp:30,duty:40},{temp:50,duty:45},{temp:65,duty:65},{temp:75,duty:85},{temp:85,duty:100}],lighting:{mode:'preserve',color:'#67c8df',brightness:70}}))},events:[{time:'1793912400',message:'Controller connected'}],uptime_secs:150});
const flush = () => new Promise(resolve=>setImmediate(resolve));
async function setup() {
  const dom = new JSDOM(html,{runScripts:'outside-only',url:'http://127.0.0.1:19273'});
  const window=dom.window,document=window.document,requests=[];
  let backend=fresh(),failure=false;
  Object.defineProperty(document,'hidden',{get:()=>false});
  window.AbortSignal=global.AbortSignal;window.requestAnimationFrame=fn=>fn();
  window.setTimeout=()=>1;window.clearTimeout=()=>{};
  window.fetch=async(url,options)=>{
    requests.push({url,options});
    if(failure)throw new Error('Connection lost');
    if(url==='/api/devices')return {ok:true,json:async()=>({devices:[{serial:'test-v2',model_name:'UNI FAN SL V2',product_id:0xa103,hardware_tested:false,max_fans_per_bank:6}],models:[]})};
    if(url==='/api/config')backend.config=JSON.parse(options.body);
    if(url==='/api/emergency')backend.emergency=true;
    if(url==='/api/resume')backend.emergency=false;
    return {ok:true,json:async()=>structuredClone(backend)};
  };
  window.eval(script);await flush();
  return {window,document,requests,get backend(){return backend;},set backend(value){backend=value;},set failure(value){failure=value;},close:()=>dom.window.close(),async poll(){document.dispatchEvent(new window.Event('visibilitychange'));await flush();},input(id,value,type='input'){const el=document.getElementById(id);el.value=value;el.dispatchEvent(new window.Event(type,{bubbles:true}));},click(selector){document.querySelector(selector).click();}};
}

test('renders actual telemetry and no fake RPM when disconnected or stale',async()=>{
  const app=await setup();try{
    assert.equal(app.document.querySelectorAll('.channel-card').length,4);
    assert.match(app.document.getElementById('cpu-temp').textContent,/43/);
    assert.match(app.document.querySelector('.channel-readout').textContent,/900/);
    app.failure=true;await app.poll();
    assert.match(app.document.getElementById('notice').textContent,/Lost contact/);
    assert.doesNotMatch(app.document.querySelector('.channel-readout').textContent,/900/);
    assert.equal(app.document.querySelectorAll('.fan-unit.running').length,0);
    app.failure=false;app.backend.controller.connected=false;await app.poll();
    assert.match(app.document.querySelector('.channel-bottom').textContent,/Disconnected/);
  }finally{app.close();}
});

test('retains drafts across polls and saves the complete config with mutation header',async()=>{
  const app=await setup();try{
    app.input('bank-name','Top exhaust');await app.poll();
    assert.equal(app.document.getElementById('bank-name').value,'Top exhaust');
    assert.equal(app.document.getElementById('save-bar').hidden,false);
    app.click('#save');await flush();
    const request=app.requests.find(r=>r.url==='/api/config');
    assert.equal(request.options.headers['X-W-Connect'],'1');
    const saved=JSON.parse(request.options.body);
    assert.equal(saved.channels.length,4);assert.equal(saved.channels[0].name,'Top exhaust');
    assert.equal(app.document.getElementById('save-bar').hidden,true);
  }finally{app.close();}
});

test('full speed and resume retain unsaved edits and report queued requests',async()=>{
  const app=await setup();try{
    app.input('bank-name','Uncommitted name');
    app.click('#emergency');await flush();
    assert.ok(app.requests.some(r=>r.url==='/api/emergency'));
    assert.match(app.document.getElementById('emergency').textContent,/Resume profile/);
    assert.equal(app.document.getElementById('bank-name').value,'Uncommitted name');
    assert.equal(app.document.getElementById('save-bar').hidden,false);
    assert.match(app.document.getElementById('toast').textContent,/requested/);
    app.click('#emergency');await flush();
    assert.ok(app.requests.some(r=>r.url==='/api/resume'));
    assert.match(app.document.getElementById('emergency').textContent,/Full speed/);
    assert.equal(app.document.getElementById('bank-name').value,'Uncommitted name');
  }finally{app.close();}
});

test('fixed mode, presets and curve values respect the cooling floor',async()=>{
  const app=await setup();try{
    app.click('[data-mode="fixed"]');assert.equal(app.document.getElementById('fixed-container').hidden,false);
    assert.equal(app.document.getElementById('fixed-duty').min,'40');
    app.input('fixed-duty','65');assert.equal(app.document.getElementById('fixed-value').textContent,'65');
    app.click('[data-mode="curve"]');app.click('[data-preset="quiet"]');
    const first=app.document.querySelector('[data-point-duty="0"]');first.value='10';first.dispatchEvent(new app.window.Event('change',{bubbles:true}));
    assert.equal(app.document.querySelector('[data-point-duty="0"]').value,'40');
    const last=app.document.querySelectorAll('[data-point-duty]');assert.equal(last[last.length-1].disabled,true);
    app.click('#save');await flush();
    const saved=JSON.parse(app.requests.find(r=>r.url==='/api/config').options.body);
    assert.equal(saved.channels[0].fixed_duty,65);assert.equal(saved.channels[0].curve[0].duty,40);
    assert.equal(saved.channels[0].curve.at(-1).duty,100);
  }finally{app.close();}
});

test('demo mode describes real temperatures and unsupported lighting is disabled',async()=>{
  const app=await setup();try{
    app.backend.demo=true;app.backend.controller.lighting_supported=false;await app.poll();
    assert.match(app.document.getElementById('notice').textContent,/readings come from this machine/);
    assert.doesNotMatch(app.document.getElementById('notice').textContent,/simulated/);
    assert.equal(app.document.getElementById('lighting-controls').disabled,true);
    assert.equal(app.document.getElementById('lighting-unsupported').hidden,false);
    assert.match(app.document.querySelector('.event-log time').textContent,/\d/);
    assert.doesNotMatch(app.document.querySelector('.event-log time').textContent,/1793912400/);
  }finally{app.close();}
});

test('critical temperature highlights full-speed protection instead of implying the curve is applied',async()=>{
  const app=await setup();try{
    app.backend.config.channels[0].source='cpu';
    app.backend.sensors.cpu=43;app.backend.sensors.gpu=86;await app.poll();
    assert.match(app.document.getElementById('notice').textContent,/Temperature protection/);
    assert.match(app.document.getElementById('notice').textContent,/All banks request 100%/);
    assert.equal(app.document.getElementById('live-point-label').textContent,'100% safety override');
  }finally{app.close();}
});


test('polling preserves bank focus, readout nodes, and chart keyboard focus',async()=>{
 const app=await setup();try{const card=app.document.querySelector('.channel-card');const readout=card.querySelector('.rpm-number');card.focus();await app.poll();assert.equal(app.document.activeElement,card);assert.equal(card.querySelector('.rpm-number'),readout);const point=app.document.querySelector('.chart-point');point.focus();await app.poll();assert.equal(app.document.activeElement,point);}finally{app.close();}
});
test('settings preferences persist independently and interval changes can be undone',async()=>{
 const app=await setup();try{app.click('[data-view="system"]');await flush();app.input('theme-preference','dark','change');assert.equal(app.document.documentElement.dataset.theme,'dark');app.input('motion-preference','reduce','change');assert.equal(app.document.documentElement.dataset.motion,'reduce');assert.equal(app.document.getElementById('save-bar').hidden,true);app.input('poll-interval','5000','change');assert.equal(app.document.getElementById('save-bar').hidden,false);app.click('#undo');assert.equal(app.document.getElementById('poll-interval').value,'2000');assert.equal(app.document.getElementById('save-bar').hidden,true);assert.equal(app.requests.filter(r=>r.options.method==='POST').length,0);}finally{app.close();}
});
test('import rejects unsafe data and stages a valid profile without writing hardware',async()=>{
 const app=await setup();try{const input=app.document.getElementById('profile-file');const profile={format:'w-connect-profile',version:1,config:fresh().config};profile.config.channels[0].fixed_duty=0;async function choose(value){Object.defineProperty(input,'files',{configurable:true,value:[{size:500,text:async()=>JSON.stringify(value)}]});input.dispatchEvent(new app.window.Event('change'));await flush();}await choose(profile);assert.match(app.document.getElementById('settings-feedback').textContent,/invalid bank/);assert.equal(app.document.getElementById('save-bar').hidden,true);profile.config.channels[0].fixed_duty=60;profile.config.channels[0].name='Imported';await choose(profile);assert.equal(app.document.getElementById('bank-name').value,'Imported');assert.equal(app.document.getElementById('save-bar').hidden,false);assert.equal(app.requests.filter(r=>r.options.method==='POST').length,0);}finally{app.close();}
});

test('lighting bank selection and numeric curve edits preserve their focused controls',async()=>{
 const app=await setup();try{app.click('[data-view="lighting"]');const tab=app.document.querySelectorAll('#lighting-tabs button')[2];tab.focus();tab.click();assert.equal(app.document.activeElement,tab);await app.poll();assert.equal(app.document.activeElement,tab);app.click('[data-view="cooling"]');const input=app.document.querySelector('[data-point-temp="1"]');input.focus();input.value='51';input.dispatchEvent(new app.window.Event('change',{bubbles:true}));assert.equal(app.document.activeElement,input);}finally{app.close();}
});

test('untested-controller selection is explicit and imported profiles cannot enable it',async()=>{
 const app=await setup();try{app.click('[data-view="system"]');await flush();assert.equal(app.document.getElementById('allow-untested').checked,false);const allow=app.document.getElementById('allow-untested');allow.checked=true;allow.dispatchEvent(new app.window.Event('change'));app.input('controller-serial','test-v2','change');assert.equal(app.document.querySelectorAll('#fan-count option').length,6);assert.equal(app.requests.filter(r=>r.options.method==='POST').length,0);app.click('#discard');assert.equal(app.document.getElementById('allow-untested').checked,false);const profile={format:'w-connect-profile',version:1,config:fresh().config};profile.config.controller={allow_untested:true,serial:'test-v2'};profile.config.channels[0].name='Imported';const input=app.document.getElementById('profile-file');Object.defineProperty(input,'files',{value:[{size:500,text:async()=>JSON.stringify(profile)}]});input.dispatchEvent(new app.window.Event('change'));await flush();assert.equal(app.document.getElementById('allow-untested').checked,false);assert.equal(app.document.getElementById('controller-serial').value,'');}finally{app.close();}
});

test('a controller capability change refreshes fan count without losing draft edits',async()=>{
 const app=await setup();try{app.input('bank-name','Draft name');app.backend.controller.max_fans_per_bank=6;app.backend.controller.name='SL V2';app.backend.controller.hardware_tested=false;await app.poll();assert.equal(app.document.querySelectorAll('#fan-count option').length,6);assert.equal(app.document.getElementById('bank-name').value,'Draft name');assert.match(app.document.getElementById('notice').textContent,/Untested driver/);}finally{app.close();}
});

test('status transitions keep one disclosure and preserve unsaved edits',async()=>{
  const app=await setup();try{
    const disclosure=app.document.getElementById('status-control');
    const status=app.document.getElementById('connection');
    app.input('bank-name','Uncommitted name');
    assert.match(status.textContent,/Connected/);
    app.backend.sensors.cpu=90;await app.poll();
    assert.match(status.textContent,/Thermal limit/);
    assert.equal(app.document.querySelectorAll('#status-control').length,1);
    assert.strictEqual(app.document.getElementById('status-control'),disclosure);
    assert.strictEqual(app.document.getElementById('connection'),status);
    app.backend.sensors.cpu=43;app.backend.sensors.gpu=null;await app.poll();
    assert.match(status.textContent,/Sensor missing/);
    app.failure=true;await app.poll();assert.match(status.textContent,/Connection lost/);
    app.failure=false;app.backend.sensors.gpu=39;await app.poll();
    assert.match(status.textContent,/Connected/);
    assert.equal(status.classList.contains('error'),false);
    assert.equal(status.classList.contains('warning'),false);
    disclosure.open=true;
    app.document.dispatchEvent(new app.window.KeyboardEvent('keydown',{key:'Escape',bubbles:true}));
    assert.equal(disclosure.open,false);
    disclosure.open=true;app.click('#page-title');assert.equal(disclosure.open,false);
    assert.equal(app.document.getElementById('bank-name').value,'Uncommitted name');
    assert.equal(app.requests.filter(r=>r.options?.method==='POST').length,0);
  }finally{app.close();}
});

test('compact bank selector retains drafts and range progress matches controls',async()=>{
  const app=await setup();try{
    app.click('[data-view="lighting"]');app.input('lighting-mode','static','change');
    app.input('lighting-brightness','50');
    assert.equal(app.document.getElementById('lighting-brightness').style.getPropertyValue('--range-fill'),'50%');
    app.input('lighting-bank-select','2','change');
    assert.equal(app.document.getElementById('bank-name').value,'Bank 3');
    app.input('lighting-bank-select','0','change');
    assert.equal(app.document.getElementById('lighting-brightness').value,'50');
    app.click('[data-view="cooling"]');app.click('[data-mode="fixed"]');
    app.input('fixed-duty','70');
    assert.equal(app.document.getElementById('fixed-duty').style.getPropertyValue('--range-fill'),'50%');
    await app.poll();assert.equal(app.document.getElementById('fixed-duty').value,'70');
    assert.equal(app.requests.filter(r=>r.options?.method==='POST').length,0);
  }finally{app.close();}
});
