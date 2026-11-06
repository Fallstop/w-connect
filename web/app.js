'use strict';
(() => {
  const $ = id => document.getElementById(id);
  const MIN_DUTY = 40;
  const presets = {
    quiet: [{temp:30,duty:40},{temp:50,duty:40},{temp:65,duty:55},{temp:75,duty:75},{temp:85,duty:100}],
    balanced: [{temp:30,duty:40},{temp:50,duty:45},{temp:65,duty:65},{temp:75,duty:85},{temp:85,duty:100}],
    performance: [{temp:30,duty:45},{temp:45,duty:60},{temp:60,duty:80},{temp:75,duty:100},{temp:85,duty:100}]
  };
  let state = null, draft = null, dirty = false, selected = 0, view = 'cooling', busy = false;
  let failure = '', toastTimer, dragging = null, pollingTimer, pollBusy = false, revision = 0;
  let undoStack = [], lastDraft = null, awaitingApply = null, catalog = null, discovering = false;
  const mediaTheme = window.matchMedia?.('(prefers-color-scheme: dark)');
  const mediaMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)');
  const preferences = (()=>{try{return JSON.parse(localStorage.getItem('w-connect-preferences')) || {};}catch{return {};}})();
  function applyPreferences() {
    document.documentElement.dataset.theme = preferences.theme === 'dark' || (preferences.theme !== 'light' && mediaTheme?.matches) ? 'dark' : 'light';
    document.documentElement.dataset.motion = preferences.motion === 'reduce' || (preferences.motion !== 'full' && mediaMotion?.matches) ? 'reduce' : 'full';
    $('theme-preference').value = preferences.theme || 'system'; $('motion-preference').value = preferences.motion || 'system';
  }
  applyPreferences(); mediaTheme?.addEventListener('change',applyPreferences); mediaMotion?.addEventListener('change',applyPreferences);
  const clone = value => JSON.parse(JSON.stringify(value));
  const esc = value => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const finite = value => typeof value === 'number' && Number.isFinite(value);
  const fmt = (value, digits = 0) => finite(value) ? value.toLocaleString(undefined, {maximumFractionDigits:digits}) : '—';
  const clamp = (value,min,max) => Math.min(max,Math.max(min,value));
  const channel = () => draft?.channels[selected];
  const actual = () => state?.channels?.find(c => c.index === selected) ?? state?.channels?.[selected];
  function toast(message, error = false) {
    clearTimeout(toastTimer); $('toast').textContent = message; $('toast').classList.toggle('error',error); $('toast').hidden = false;
    toastTimer = setTimeout(() => { $('toast').hidden = true; }, error ? 9000 : 4000);
  }
  async function request(path, body) {
    const response = await fetch(path, {method:body === undefined ? 'GET' : 'POST', headers:body === undefined ? {} : {'Content-Type':'application/json','X-W-Connect':'1'}, body:body === undefined ? undefined : JSON.stringify(body), signal:AbortSignal.timeout(7000), cache:'no-store'});
    let data;
    try { data = await response.json(); } catch { throw new Error('The local service returned an unreadable response.'); }
    if (!response.ok || data.error && !data.controller) throw new Error(data.error || `Request failed (${response.status}).`);
    return data;
  }
  function dirtyChanged(record = true) {
    const next = JSON.stringify(draft);
    if (record && !dragging && lastDraft && next !== lastDraft && next !== JSON.stringify(state?.config)) { undoStack.push(JSON.parse(lastDraft)); if (undoStack.length > 40) undoStack.shift(); }
    if (!dragging) lastDraft = next;
    dirty = JSON.stringify(draft) !== JSON.stringify(state?.config);
    $('save-bar').hidden = !dirty;
    document.body.classList.toggle('has-draft',dirty);
    $('save').disabled = busy || !dirty;
    $('discard').disabled = busy;
    if (!dirty) undoStack = [];
    $('undo').disabled = busy || !undoStack.length;
    const changed = draft?.channels.filter((c,i)=>JSON.stringify(c)!==JSON.stringify(state?.config.channels[i])).length || 0;
    $('change-summary').textContent = changed ? `${changed} bank${changed===1?'':'s'} changed` : JSON.stringify(draft?.controller)!==JSON.stringify(state?.config.controller) ? 'Controller settings changed' : 'Control interval changed';
    renderPresets();
  }
  function criticalTemperature() { return finite(state?.sensors?.cpu) && state.sensors.cpu >= 85 || finite(state?.sensors?.gpu) && state.sensors.gpu >= 85; }
  function selectedTemperature() {
    const sensors = state?.sensors || {}, source = channel()?.source;
    if (source === 'cpu') return sensors.cpu;
    if (source === 'gpu') return sensors.gpu;
    return finite(sensors.cpu) && finite(sensors.gpu) ? Math.max(sensors.cpu,sensors.gpu) : null;
  }
  let fanId=0;
  function lightingLayout() {
    const pid=state?.controller?.pid;
    const id=typeof pid==='number'?pid:Number.parseInt(pid,16);
    return [0xa100,0x7750,0xa103,0xa105].includes(id)?'sl-frame':'ring';
  }
  function hardwareBrightness(value) {return value===0?0:clamp(Math.round(value/25)*25,25,100);}
  function fanSVG(index, layout, mode) {
    const id=`fan-${++fanId}`;
    const edge=layout==='sl-frame';
    const shape=edge?'<path d="M3 15L12 7H52L61 15M3 49L12 57H52L61 49"/>':'<circle cx="32" cy="32" r="25.4"/>';
    const spectrum=mode==='rainbow'?`url(#${id}-spectrum)`:'currentColor';
    return `<svg class="fan-unit" viewBox="0 0 64 64" aria-hidden="true" style="--phase:${-index*1.25}s">
      <defs><linearGradient id="${id}-frame" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#3c3c43"/><stop offset=".45" stop-color="#25252b"/><stop offset="1" stop-color="#1c1c21"/></linearGradient><linearGradient id="${id}-blade" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#50505a"/><stop offset=".45" stop-color="#383840"/><stop offset="1" stop-color="#27272e"/></linearGradient><radialGradient id="${id}-hub" cx=".35" cy=".25"><stop stop-color="#505059"/><stop offset="1" stop-color="#24242b"/></radialGradient><clipPath id="${id}-aperture"><circle cx="32" cy="32" r="23.5"/></clipPath><linearGradient id="${id}-spectrum" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="64" y2="0"><stop stop-color="#ff4848"/><stop offset=".17" stop-color="#ffcb45"/><stop offset=".34" stop-color="#79e864"/><stop offset=".5" stop-color="#45dfe8"/><stop offset=".67" stop-color="#6387ff"/><stop offset=".84" stop-color="#da65fa"/><stop offset="1" stop-color="#ff4848"/></linearGradient></defs>
      <rect x="1" y="1" width="62" height="62" rx="6" fill="url(#${id}-frame)" stroke="#52525c" stroke-width=".55"/><rect x="2" y="2" width="60" height="60" rx="5" fill="none" stroke="#6c6c77" stroke-opacity=".3" stroke-width=".35"/><circle cx="32" cy="32" r="25" fill="#0f0f13"/><circle cx="32" cy="32" r="23.7" fill="#17171c" stroke="#55555f" stroke-width=".35"/>
      <g clip-path="url(#${id}-aperture)"><g class="fan-blades">${Array.from({length:7},(_,i) => `<path transform="rotate(${i*360/7+index*17} 32 32)" d="M28 29C20 27 10 24 9 19C10 14 16 11 21 11C24 15 28 19 33 23L35 28Z" fill="url(#${id}-blade)" stroke="#5a5a65" stroke-width=".16"/>`).join('')}</g></g><circle cx="32" cy="32" r="8.2" fill="#121217"/><circle cx="32" cy="32" r="7.3" fill="url(#${id}-hub)" stroke="#62626d" stroke-width=".25"/>
      <g class="led-diffuser" fill="none" stroke="#55555e" stroke-width="${edge?2.1:1.3}" stroke-linejoin="round">${shape}</g>
      <g class="led-motion"><g class="led-output" fill="none" stroke="${spectrum}" stroke-linejoin="round"><g class="led-glow">${shape}</g><g class="led-strip" stroke-width="${edge?1.7:1.1}">${shape}</g></g></g>
      ${[[5,5],[59,5],[5,59],[59,59]].map(([x,y])=>`<circle cx="${x}" cy="${y}" r="1.2" fill="#111115" stroke="#62626d" stroke-width=".25"/>`).join('')}<path d="M0 28h2v8H0m62-8h2v8h-2" fill="#55555e"/></svg>`;
  }
  function renderChannels() {
    if (!draft) return;
    const container=$('channels');
    if(container.children.length!==draft.channels.length) container.innerHTML=draft.channels.map((_,i)=>`<button class="channel-card" data-channel="${i}"><div class="channel-card-top"><span class="channel-number">${String(i+1).padStart(2,'0')}</span><h3></h3></div><div class="channel-readout"><strong><span class="rpm-number"></span><small>RPM</small></strong></div><div class="channel-bottom"><span class="card-mode"></span><span class="channel-duty"></span></div></button>`).join('');
    draft.channels.forEach((config,i)=>{
      const live=state.channels?.find(c=>c.index===i)||{}, card=container.children[i], connected=state.controller?.connected && !failure;
      const rpm=connected ? live.rpm : null;
      card.classList.toggle('selected',selected===i); card.classList.toggle('idle',rpm===0); card.setAttribute('aria-pressed',String(selected===i));card.setAttribute('aria-label',`Select ${config.name || `Channel ${i+1}`}`);
      card.querySelector('h3').textContent=config.name;card.querySelector('h3').title=config.name;
      card.querySelector('.rpm-number').textContent=fmt(rpm);card.querySelector('.channel-duty').textContent=connected?`${fmt(live.duty)}%`:'—';
      card.querySelector('.card-mode').textContent=failure?'Stale':!connected?'Disconnected':rpm===0?'No RPM signal':live.mode==='fixed'?'Fixed':'Curve';
      card.title=rpm===0?'No speed signal. This port may be empty, disconnected, or stalled.':`${config.name}: ${fmt(rpm)} RPM`;
    });
    const tabs=$('lighting-tabs');
    if(tabs.children.length!==draft.channels.length) tabs.innerHTML=draft.channels.map((_,i)=>`<button data-channel="${i}"></button>`).join('');
    draft.channels.forEach((c,i)=>{const button=tabs.children[i];button.textContent=c.name;button.classList.toggle('active',selected===i);button.setAttribute('aria-pressed',String(selected===i));});
    const chooser=$('lighting-bank-select');
    if(chooser.options.length!==draft.channels.length)chooser.replaceChildren(...draft.channels.map((_,i)=>new Option('',String(i))));
    draft.channels.forEach((c,i)=>{chooser.options[i].textContent=c.name;});chooser.value=String(selected);
  }
  function renderTelemetry() {
    if (!state) return;
    const connected = state.controller?.connected, stale = Boolean(failure);
    $('controller-name').textContent = (state.controller?.name || 'No controller').replace(/^LianLi[- ]/i,'');
    for (const type of ['cpu','gpu']) {
      const value = stale ? null : state.sensors?.[type];
      $(type+'-temp').innerHTML = `${fmt(value)}<small>°C</small>`;
      $(type+'-temp').classList.toggle('is-hot',finite(value) && value>=85);
    }
    const source = channel()?.source;
    const missing = ['cpu','gpu'].filter(type=>(source===type || source==='max') && !finite(state.sensors?.[type]));
    let label='Connected', severity='', message=`${state.controller?.name || 'Controller'} connected. Cooling continues when this page closes.`;
    if (failure) {label='Connection lost';severity='error';message='Lost contact with W-Connect. Readings are unavailable. Reconnecting automatically.';}
    else if (state.demo) {label='Preview mode';message='No hardware commands are sent. Temperature readings come from this machine.';}
    else if (!connected) {label='No controller';severity='error';message=state.controller?.error || 'Check the USB connection and device permissions.';}
    else if (state.controller?.error) {label='Check controller';severity='error';message=state.controller.error;}
    else if (criticalTemperature()) {label='Thermal limit · 100%';severity='warning';message='Temperature protection: CPU or GPU ≥85°C. All banks request 100% until temperatures fall.';}
    else if (state.emergency) {label='Full speed';severity='warning';message='Full-speed override is active. Select Resume profile to use your saved settings.';}
    else if (missing.length) {label='Sensor missing · 100%';severity='warning';message=`${missing.map(x=>x.toUpperCase()).join(' and ')} temperature unavailable. Affected banks request full speed until readings return.`;}
    else if (state.controller?.hardware_tested===false) {label='Untested controller';severity='warning';message='Untested driver. This model has protocol evidence but no physical hardware validation. Check temperatures and RPM after changes.';}
    $('connection').classList.remove('is-connecting');
    $('connection').classList.toggle('warning',severity==='warning');
    $('connection').classList.toggle('error',severity==='error');
    const statusLabel=$('connection').lastElementChild;
    if(statusLabel.textContent!==label)statusLabel.textContent=label;
    $('connection').setAttribute('aria-label',`${label}. Show status details`);
    $('notice').textContent=message; $('notice').hidden=false;
    $('emergency').disabled=busy || !state;
    if(!busy){$('emergency').textContent=state.emergency?'Resume profile':'Full speed';$('emergency').title=state.emergency?'Resume saved cooling settings':'Request 100% on all fan banks';}
    const live=actual();
    $('selected-rpm').innerHTML=`${fmt(connected && !stale ? live?.rpm : null)} <small>RPM</small>`;
    $('selected-target').innerHTML=`${fmt(connected && !stale ? live?.target : null)} <small>%</small>`;
    $('lighting-controls').disabled=!state.controller?.lighting_supported;
    $('lighting-unsupported').hidden=Boolean(state.controller?.lighting_supported);
    renderChannels();
    if(!document.activeElement?.closest('#curve-chart'))renderGraph();
    renderSystem();renderApplyStatus();
  }
  function renderFanCountOptions(){
    if(!channel())return;
    const maxFans=targetMaxFans();
    $('fan-count').innerHTML=Array.from({length:Math.max(maxFans,channel().fan_count)},(_,i)=>`<option value="${i+1}"${i+1>maxFans?' disabled':''}>${i+1} fan${i?'s':''}${i+1>maxFans?' · exceeds controller limit':''}</option>`).join('');
    $('fan-count').value = channel().fan_count;
  }
  function renderEditor() {
    const config = channel(); if (!config) return;
    $('selected-label').textContent = `Port ${String(selected+1).padStart(2,'0')} · ${config.name || 'Fan bank'}`;
    $('poll-interval').value = draft.poll_ms;
    $('allow-untested').checked = Boolean(draft.controller?.allow_untested);
    renderDevices();
    $('port-label').textContent = String(selected+1).padStart(2,'0');
    $('bank-name').value = config.name || '';
    renderFanCountOptions();
    $('temperature-source').value = config.source;
    $('fixed-duty').value = config.fixed_duty;
    $('fixed-value').textContent = config.fixed_duty;
    renderRange($('fixed-duty'));
    $('curve-container').hidden = config.mode !== 'curve';
    $('fixed-container').hidden = config.mode !== 'fixed';
    $('temperature-source').disabled = false;
    document.querySelectorAll('[data-mode]').forEach(el => { el.classList.toggle('active', el.dataset.mode === config.mode); el.setAttribute('aria-pressed',el.dataset.mode === config.mode); });
    $('copy-all').textContent='Copy to all banks';
    $('curve-title').textContent=config.mode==='curve'?'Fan curve':'Fixed speed';
    document.querySelector('.preset-buttons').hidden = config.mode !== 'curve';
    renderPresets(); renderPointTable(); renderGraph(); renderLighting();
  }
  function renderPresets() {
    document.querySelectorAll('[data-preset]').forEach(el => {
      const same = JSON.stringify(channel()?.curve) === JSON.stringify(presets[el.dataset.preset]);
      el.classList.toggle('active',same); el.setAttribute('aria-pressed',same);
    });
  }
  const chart = {left:37,right:643,top:19,bottom:239,minTemp:20,maxTemp:90,minDuty:40,maxDuty:100};
  const x = temp => chart.left+(temp-chart.minTemp)/(chart.maxTemp-chart.minTemp)*(chart.right-chart.left);
  const y = duty => chart.bottom-(duty-chart.minDuty)/(chart.maxDuty-chart.minDuty)*(chart.bottom-chart.top);
  function interpolate(points,temp) {
    if (!points?.length || !finite(temp)) return null;
    if (temp <= points[0].temp) return points[0].duty;
    for (let i=1;i<points.length;i++) if (temp <= points[i].temp) {
      const a=points[i-1],b=points[i]; return a.duty+(b.duty-a.duty)*(temp-a.temp)/(b.temp-a.temp);
    }
    return points.at(-1).duty;
  }
  function renderGraph() {
    const config = channel(); if (!config?.curve?.length || dragging !== null || $('cooling-view').hidden) return;
    const width = Math.max(300,$('curve-chart').clientWidth || 660);
    chart.right=width-17; chart.bottom=209; $('curve-chart').setAttribute('viewBox',`0 0 ${width} 240`);
    const points = config.curve;
    let html = '';
    for (const duty of [40,60,80,100]) html += `<line class="chart-grid" x1="${chart.left}" x2="${chart.right}" y1="${y(duty)}" y2="${y(duty)}"/><text class="chart-axis" x="25" y="${y(duty)+3}" text-anchor="end">${duty}</text>`;
    for (let temp=20;temp<=90;temp+=10) html += `<line class="chart-grid" x1="${x(temp)}" x2="${x(temp)}" y1="${chart.top}" y2="${chart.bottom}"/><text class="chart-axis" x="${x(temp)}" y="230" text-anchor="middle">${temp}</text>`;
    const path = `M${x(20)},${y(points[0].duty)} `+points.map(p=>`L${x(p.temp)},${y(p.duty)}`).join(' ')+` L${x(90)},${y(points.at(-1).duty)}`;
    html += `<path class="chart-area" d="${path} L${x(90)},${chart.bottom} L${x(20)},${chart.bottom} Z"/><path class="chart-path" d="${path}"/>`;
    const temp = failure ? null : selectedTemperature();
    const target = interpolate(points,temp);
    if (finite(temp)) {
      const tx = x(clamp(temp,20,90)), ty = y(state.emergency || criticalTemperature() ? 100 : target);
      html += `<line class="chart-live-line" x1="${tx}" x2="${tx}" y1="${ty}" y2="${chart.bottom}"/><circle class="chart-live" cx="${tx}" cy="${ty}" r="5"/>`;
      $('live-point-label').textContent = `${fmt(temp,1)}°C → ${fmt(target)}% preview`;
      if (state.emergency || criticalTemperature()) $('live-point-label').textContent = '100% safety override';
    } else $('live-point-label').textContent = 'Temperature unavailable';
    points.forEach((p,i) => {
      html += `<g data-point="${i}"><circle class="chart-point-halo" cx="${x(p.temp)}" cy="${y(p.duty)}" r="14"/><circle class="chart-point" tabindex="0" role="slider" aria-label="Curve point ${i+1}, ${p.temp} degrees Celsius. Arrow keys adjust temperature and speed." aria-valuemin="40" aria-valuemax="100" aria-valuenow="${p.duty}" aria-valuetext="${p.temp}°C, ${p.duty}%" cx="${x(p.temp)}" cy="${y(p.duty)}" r="5"/></g>`;
    });
    const focused = document.activeElement?.closest('[data-point]')?.dataset.point;
    $('curve-chart').innerHTML = html;
    if (focused !== undefined) $('curve-chart').querySelector(`[data-point="${focused}"] .chart-point`)?.focus({preventScroll:true});
  }
  function renderPointTable() {
    if (!channel()) return;
    const points=channel().curve, table=$('point-table');
    if(table.children.length!==points.length) table.innerHTML=points.map((_,i)=>`<div class="point-row"><span>${String(i+1).padStart(2,'0')}</span><label><span class="sr-only">Point ${i+1} temperature</span><input type="number" data-point-temp="${i}" step="1">°C</label><label><span class="sr-only">Point ${i+1} fan speed</span><input type="number" data-point-duty="${i}" step="1">%</label><button class="remove-point" data-remove-point="${i}" aria-label="Remove point ${i+1}">×</button></div>`).join('');
    points.forEach((p,i)=>{const row=table.children[i],temp=row.querySelector('[data-point-temp]'),duty=row.querySelector('[data-point-duty]');temp.value=p.temp;temp.min=i?points[i-1].temp+1:20;temp.max=i<points.length-1?points[i+1].temp-1:85;duty.value=p.duty;duty.min=i?points[i-1].duty:MIN_DUTY;duty.max=i<points.length-1?points[i+1].duty:100;duty.disabled=i===points.length-1;duty.title=duty.disabled?'The final point stays at 100% for safe cooling':'';row.querySelector('button').hidden=points.length<=2||i===points.length-1;});
    $('add-point').disabled = points.length >= 10;
  }
  function setPoint(index,temp,duty) {
    const points = channel().curve;
    const before=points[index-1],after=points[index+1];
    points[index] = {temp:clamp(Math.round(temp),before ? before.temp+1 : 20,after ? after.temp-1 : 85),duty:index===points.length-1 ? 100 : clamp(Math.round(duty),before ? before.duty : MIN_DUTY,after ? after.duty : 100)};
    dirtyChanged();
  }
  function renderRange(input){input.style.setProperty('--range-fill',`${(Number(input.value)-Number(input.min))/(Number(input.max)-Number(input.min))*100}%`);}
  let pickerHSV={h:0,s:0,v:100}, pickerColor=null, colorPointer=null;
  function colorToHSV(hex) {
    const [r,g,b]=hex.slice(1).match(/../g).map(v=>parseInt(v,16)/255);
    const max=Math.max(r,g,b), min=Math.min(r,g,b), d=max-min;
    const h=d===0?pickerHSV.h:((max===r?(g-b)/d:max===g?(b-r)/d+2:(r-g)/d+4)*60+360)%360;
    return {h,s:max?d/max*100:0,v:max*100};
  }
  function hsvToColor({h,s,v}) {
    const c=v/100*s/100, x=c*(1-Math.abs((h/60)%2-1)), m=v/100-c;
    const rgb=[[c,x,0],[x,c,0],[0,c,x],[0,x,c],[x,0,c],[c,0,x]][Math.floor(h/60)%6];
    return '#'+rgb.map(n=>Math.round((n+m)*255).toString(16).padStart(2,'0')).join('');
  }
  function renderColorPicker(color) {
    if(color!==pickerColor){pickerHSV=colorToHSV(color);pickerColor=color;}
    const {h,s,v}=pickerHSV;
    $('color-plane').style.setProperty('--picker-hue',`hsl(${h} 100% 50%)`);
    $('color-cursor').style.left=`${s}%`;
    $('color-cursor').style.top=`${100-v}%`;
    $('color-cursor').style.background=color;
    for(const [id,value,unit] of [['color-hue',h,'degrees'],['color-saturation',s,'percent'],['color-value',v,'percent']]) {
      $(id).value=Math.round(value);$(id).setAttribute('aria-valuetext',`${Math.round(value)} ${unit}`);
    }
  }
  function updatePicker() {
    pickerColor=hsvToColor(pickerHSV);channel().lighting.color=pickerColor;
    dirtyChanged();renderLighting();
  }
  function pickPosition(event) {
    const rect=$('color-plane').getBoundingClientRect();
    if(!rect.width||!rect.height)return;
    pickerHSV.s=clamp((event.clientX-rect.left)/rect.width*100,0,100);
    pickerHSV.v=100-clamp((event.clientY-rect.top)/rect.height*100,0,100);
    updatePicker();
  }
  function renderLighting() {
    const config = channel(); if (!config) return;
    const lighting = config.lighting || {mode:'preserve',color:'#67c8df',brightness:70};
    const brightness=hardwareBrightness(lighting.brightness), layout=lightingLayout();
    $('lighting-unsupported').hidden = Boolean(state.controller?.lighting_supported);
    $('lighting-controls').disabled = !state.controller?.lighting_supported;
    $('lighting-mode').value = lighting.mode;
    $('lighting-color').value = lighting.color;
    renderColorPicker(lighting.color);
    if (document.activeElement !== $('color-hex')) {$('color-hex').value = lighting.color.toUpperCase();$('color-hex').removeAttribute('aria-invalid');}
    $('color-label').textContent = lighting.color.toUpperCase();
    $('lighting-brightness').value = brightness;renderRange($('lighting-brightness'));
    $('brightness-label').textContent = `${brightness}%`;
    $('lighting-brightness').title='Hardware brightness uses 25% steps';
    document.querySelectorAll('[data-color]').forEach(el=>el.setAttribute('aria-pressed',String(el.dataset.color.toLowerCase()===lighting.color.toLowerCase())));
    $('color-controls').hidden = !['static','breathing'].includes(lighting.mode);
    $('lighting-brightness').disabled = ['preserve','off'].includes(lighting.mode);
    $('lighting-preview').classList.toggle('is-off',lighting.mode==='off');
    $('lighting-preview').classList.toggle('is-preserve',lighting.mode==='preserve');
    $('lighting-preview').classList.toggle('is-rainbow',lighting.mode==='rainbow');
    $('lighting-preview').classList.toggle('is-breathing',lighting.mode==='breathing');
    $('lighting-preview').dataset.layout=layout;
    $('lighting-preview').style.setProperty('--light-color',lighting.color);
    $('lighting-preview').style.setProperty('--led-opacity',brightness/100);
    $('lighting-preview').setAttribute('aria-label',`${layout==='sl-frame'?'SL frame lighting':'Simplified lighting layout'}, ${config.fan_count} fans. ${lighting.mode==='preserve'?'Current lighting is not read back':lighting.mode==='off'?'Lights off':`${lighting.mode} lighting preview, ${brightness}% brightness`}.`);
    $('lighting-preview').innerHTML = `<div class="fan-bank">${Array.from({length:clamp(config.fan_count || 1,1,6)},(_,index)=>fanSVG(index,layout,lighting.mode)).join('')}</div><span class="preview-caption">${lighting.mode==='preserve'?'Current hardware lighting is not read back':''}</span>`;
  }
  function renderSystem() {
    const rows = [['Driver verification',state.controller?.hardware_tested === true ? 'Hardware-tested on SL v1 a100' : state.controller?.connected ? 'Protocol-verified · hardware untested' : 'No active controller'],['Application',`W-Connect ${state.version || ''}`],['Platform',state.platform || 'Unknown'],['Controller',state.controller?.name || 'Unavailable'],['USB identifier',`${usbHex(state.controller?.vid)}:${usbHex(state.controller?.pid)}`],['Sensor source',state.sensors?.source || 'Unavailable'],['Polling interval',`${(state.config?.poll_ms || 2000)/1000} seconds`],['Running for',formatUptime(state.uptime_secs)],['Connection',failure ? 'Interrupted' : state.controller?.connected ? 'Connected' : 'Unavailable']];
    $('system-details').innerHTML = rows.map(([a,b])=>`<div><dt>${esc(a)}</dt><dd>${esc(b)}</dd></div>`).join('');
    $('event-log').innerHTML = state.events?.length ? [...state.events].reverse().slice(0,40).map(event=>`<li><time>${esc(formatTime(event.time))}</time>${esc(event.message)}</li>`).join('') : '<li class="muted">No recent activity.</li>';
  }
  function usbHex(value) { return finite(value) ? value.toString(16).padStart(4,'0') : value || '—'; }
  function formatTime(value) { const numeric = Number(value); const date = new Date(Number.isFinite(numeric) && String(value).trim() !== '' ? numeric * 1000 : value); return Number.isNaN(date.getTime()) ? String(value || '') : date.toLocaleTimeString(); }
  function formatUptime(value) { if (!finite(value)) return '—'; const h=Math.floor(value/3600),m=Math.floor(value%3600/60); return h ? `${h}h ${m}m` : m ? `${m}m` : `${Math.floor(value)}s`; }
  function showView(next) {
    const changedView = view !== next; view=next;
    if (next==='system' && !catalog && !discovering) discoverDevices();
    if (changedView) {$(next+'-view').classList.remove('view-enter'); requestAnimationFrame(()=>$(next+'-view').classList.add('view-enter'));}
    $('page-title').textContent={cooling:'Cooling',lighting:'Lighting',system:'Settings'}[next];
    document.querySelectorAll('[data-view]').forEach(el => { el.classList.toggle('active',el.dataset.view===next); if (el.dataset.view===next) el.setAttribute('aria-current','page'); else el.removeAttribute('aria-current'); });
    for(const name of ['cooling','lighting','system']) $(name+'-view').hidden=!state || name!==next;
    if(changedView){$('main-content').scrollTop=0;if(next==='cooling')renderGraph();}
  }
  function acceptState(data, reset = false) {
    if (!data.channels?.length && data.uptime_secs < 2 && !data.demo) return;
    if (!data.config?.channels?.length) throw new Error('The local service returned an incomplete configuration.');
    const configChanged = JSON.stringify(state?.config)!==JSON.stringify(data.config);
    const capabilitiesChanged = JSON.stringify(state?.controller)!==JSON.stringify(data.controller);
    state=data; failure='';
    const editingField = document.activeElement?.matches('input, select') || dragging !== null;
    const shouldReset = !draft || reset || (!dirty && !editingField && configChanged);
    if (shouldReset) { draft=clone(state.config); selected=Math.min(selected,draft.channels.length-1); dirty=false; renderEditor(); dirtyChanged(); }
    if(capabilitiesChanged){renderFanCountOptions();if(!shouldReset)renderLighting();}
    $('loading').hidden=true;
    showView(view); renderTelemetry();
  }
  async function poll() {
    clearTimeout(pollingTimer);
    if (pollBusy || busy) { pollingTimer=setTimeout(poll,1000); return; }
    pollBusy=true;
    const startedRevision = revision;
    try {
      const data=await request('/api/state');
      if (!busy && startedRevision === revision) acceptState(data);
    } catch(error) {
      failure=error.message;
      if (state) renderTelemetry();
      else { $('notice').textContent='Start W-Connect to reconnect to the local service.'; $('notice').hidden=false; $('connection').classList.add('error'); $('connection').lastElementChild.textContent='Service unavailable'; $('loading').querySelector('p').textContent='Service unavailable. Retrying automatically.'; $('retry-connection').hidden=false; }
    } finally { pollBusy=false; pollingTimer=setTimeout(poll,state?.config?.poll_ms || 2000); }
  }
  function validate() {
    if ($('color-hex').getAttribute('aria-invalid') === 'true') return 'Correct the hex color before saving.';
    for (const [i,c] of draft.channels.entries()) {
      if (c.fan_count>targetMaxFans()) return `This controller supports up to ${targetMaxFans()} fans per bank.`;
      if (!c.name.trim()) return `Give bank ${i+1} a name.`;
      const p=c.curve;
      if (!Array.isArray(p) || p.length<2) return 'A curve needs at least two points.';
      if (p.some((v,j)=>!finite(v.temp)||!finite(v.duty)||v.temp<20||v.temp>85||v.duty<40||v.duty>100||(j>0&&(v.temp<=p[j-1].temp||v.duty<p[j-1].duty)))) return 'Curve points must rise in temperature and never decrease in speed, within 20–85°C and 40–100%.';
      if (p.at(-1).duty!==100) return 'Each curve must reach 100% by 85°C.';
    }
    return '';
  }
  function setEditorBusy(value) {
    for (const name of ['cooling','lighting','system']) $(name+'-view').inert = value;
    $('save').disabled=value; $('discard').disabled=value;
  }
  async function save() {
    if (busy || !draft || !dirty) return;
    const error=validate(); if(error){toast(error,true);return;}
    busy=true; revision++; setEditorBusy(true); $('save').textContent='Saving…'; dirtyChanged(); $('emergency').disabled=true;
    try { const data=await request('/api/config',clone(draft)); awaitingApply=data.requested_revision ?? null; acceptState(data,true);  }
    catch(error){toast(error.message,true);} finally {busy=false;setEditorBusy(false);$('save').textContent='Save changes';dirtyChanged();$('emergency').disabled=false;poll();}
  }
  async function emergency() {
    if (busy || !state) return;
    const resuming = Boolean(state.emergency);
    busy=true; revision++; setEditorBusy(true); $('emergency').disabled=true; $('emergency').textContent=resuming ? 'Resuming…' : 'Requesting…';
    try { const data=await request(resuming ? '/api/resume' : '/api/emergency',{}); acceptState(data); toast(resuming ? 'Saved profile requested.' : 'Full speed requested.'); }
    catch(error){toast('Could not update cooling override: '+error.message,true);} finally {busy=false;setEditorBusy(false);$('emergency').textContent=state.emergency ? 'Resume profile' : 'Full speed';$('emergency').disabled=false;dirtyChanged();poll();}
  }
  document.addEventListener('click',event=>{
    const nav=event.target.closest('[data-view]');if(nav){showView(nav.dataset.view);return;}
    const bank=event.target.closest('[data-channel]');if(bank){selected=Number(bank.dataset.channel);renderChannels();renderEditor();renderTelemetry();return;}
    if(!draft || busy)return;
    const mode=event.target.closest('[data-mode]');if(mode){channel().mode=mode.dataset.mode;dirtyChanged();renderEditor();return;}
    const preset=event.target.closest('[data-preset]');if(preset){channel().curve=clone(presets[preset.dataset.preset]);channel().mode='curve';dirtyChanged();renderEditor();return;}
    const remove=event.target.closest('[data-remove-point]');if(remove){channel().curve.splice(Number(remove.dataset.removePoint),1);dirtyChanged();renderPointTable();renderGraph();return;}
    const swatch=event.target.closest('[data-color]');if(swatch){channel().lighting.color=swatch.dataset.color;dirtyChanged();renderLighting();}
  });
  $('lighting-bank-select').addEventListener('change',event=>{selected=Number(event.target.value);renderChannels();renderEditor();renderTelemetry();});
  $('temperature-source').addEventListener('change',event=>{channel().source=event.target.value;dirtyChanged();renderTelemetry();});
  $('bank-name').addEventListener('input',event=>{channel().name=event.target.value;dirtyChanged();renderChannels();$('selected-label').textContent=`Port ${String(selected+1).padStart(2,'0')} · ${channel().name || 'Fan bank'}`;});
  $('fan-count').addEventListener('change',event=>{channel().fan_count=Number(event.target.value);dirtyChanged();renderChannels();renderLighting();});
  $('fixed-duty').addEventListener('input',event=>{channel().fixed_duty=Number(event.target.value);$('fixed-value').textContent=event.target.value;renderRange(event.target);dirtyChanged();});
  $('copy-all').addEventListener('click',()=>{const source=clone(channel());draft.channels.forEach(c=>{c.mode=source.mode;c.source=source.source;c.curve=clone(source.curve);c.fixed_duty=source.fixed_duty;});dirtyChanged();toast('Copied to all banks.');});
  $('copy-lighting').addEventListener('click',()=>{const lighting=clone(channel().lighting);draft.channels.forEach(c=>{c.lighting=clone(lighting);});dirtyChanged();toast('Copied to all banks.');});
  $('lighting-mode').addEventListener('change',event=>{channel().lighting.mode=event.target.value;dirtyChanged();renderLighting();});
  for(const [id,key] of [['color-hue','h'],['color-saturation','s'],['color-value','v']]) {
    $(id).addEventListener('input',event=>{pickerHSV[key]=Number(event.target.value);updatePicker();$('color-keyboard-value').textContent=`${key==='s'?'Saturation':key==='v'?'Value':'Hue'} ${Math.round(pickerHSV[key])}${key==='h'?'°':'%'}`;});
    $(id).addEventListener('focus',()=>{$('color-keyboard-value').textContent=`${key==='s'?'Saturation':key==='v'?'Value':'Hue'} ${Math.round(pickerHSV[key])}${key==='h'?'°':'%'}`;});
  }
  $('color-plane').addEventListener('pointerdown',event=>{
    if(event.button!==0||$('lighting-controls').disabled)return;
    event.preventDefault();colorPointer=event.pointerId;
    $('color-plane').setPointerCapture(event.pointerId);pickPosition(event);
  });
  $('color-plane').addEventListener('pointermove',event=>{if(colorPointer===event.pointerId)pickPosition(event);});
  for(const name of ['pointerup','pointercancel','lostpointercapture'])$('color-plane').addEventListener(name,()=>{colorPointer=null;});
  $('lighting-color').addEventListener('input',event=>{channel().lighting.color=event.target.value;dirtyChanged();renderLighting();});
  $('lighting-brightness').addEventListener('input',event=>{channel().lighting.brightness=Number(event.target.value);dirtyChanged();renderLighting();});
  $('point-table').addEventListener('change',event=>{const el=event.target;if(el.dataset.pointTemp!==undefined||el.dataset.pointDuty!==undefined){const index=Number(el.dataset.pointTemp??el.dataset.pointDuty);const p=channel().curve[index],value=Number(el.value);if(el.value!==''&&finite(value))setPoint(index,el.dataset.pointTemp!==undefined?value:p.temp,el.dataset.pointDuty!==undefined?value:p.duty);renderPointTable();renderGraph();}});
  $('add-point').addEventListener('click',()=>{const points=channel().curve;if(points.length>=10)return;let index=-1,gap=1;for(let i=0;i<points.length-1;i++)if(points[i+1].temp-points[i].temp>gap){gap=points[i+1].temp-points[i].temp;index=i;}if(index===-1){toast('There is no space between the existing points.',true);return;}points.splice(index+1,0,{temp:Math.floor((points[index].temp+points[index+1].temp)/2),duty:Math.round((points[index].duty+points[index+1].duty)/2)});dirtyChanged();renderPointTable();renderGraph();});
  $('curve-chart').addEventListener('pointerdown',event=>{const group=event.target.closest('[data-point]');if(!group)return;event.preventDefault();dragging={index:Number(group.dataset.point),pointerId:event.pointerId};$('curve-chart').setPointerCapture(event.pointerId);});
  $('curve-chart').addEventListener('pointermove',event=>{if(dragging===null||event.pointerId!==dragging.pointerId)return;const point=$('curve-chart').createSVGPoint();point.x=event.clientX;point.y=event.clientY;const pos=point.matrixTransform($('curve-chart').getScreenCTM().inverse());setPoint(dragging.index,chart.minTemp+(pos.x-chart.left)/(chart.right-chart.left)*(chart.maxTemp-chart.minTemp),chart.minDuty+(chart.bottom-pos.y)/(chart.bottom-chart.top)*(chart.maxDuty-chart.minDuty));const active=dragging;dragging=null;renderGraph();dragging=active;});
  function endDrag(){if(dragging===null)return;dragging=null;dirtyChanged();renderPointTable();renderGraph();}
  $('curve-chart').addEventListener('pointerup',endDrag);$('curve-chart').addEventListener('pointercancel',endDrag);$('curve-chart').addEventListener('lostpointercapture',endDrag);
  $('curve-chart').addEventListener('keydown',event=>{const group=event.target.closest('[data-point]');if(!group||!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(event.key))return;event.preventDefault();const index=Number(group.dataset.point),p=channel().curve[index],step=event.shiftKey?5:1;setPoint(index,p.temp+(event.key==='ArrowRight'?step:event.key==='ArrowLeft'?-step:0),p.duty+(event.key==='ArrowUp'?step:event.key==='ArrowDown'?-step:0));renderPointTable();renderGraph();$('curve-chart').querySelector(`[data-point="${index}"] .chart-point`).focus();});
  $('save').addEventListener('click',save);$('emergency').addEventListener('click',emergency);
  $('discard').addEventListener('click',()=>{if(busy)return;draft=clone(state.config);dirtyChanged();renderEditor();renderTelemetry();toast('Unsaved changes discarded.');});


  function targetMaxFans(){
    const found=catalog?.devices||[],serial=draft?.controller?.serial;
    const fallback={allow_untested:false,serial:null};
    if(state?.controller?.connected && JSON.stringify(draft?.controller||fallback)===JSON.stringify(state?.config.controller||fallback)) return state.controller.max_fans_per_bank||4;
    const candidates=found.filter(d=>(!serial||d.serial===serial)&&(d.hardware_tested||draft?.controller?.allow_untested));
    return candidates.length===1?candidates[0].max_fans_per_bank||4:state?.controller?.max_fans_per_bank||4;
  }
  function renderDevices(){
    if(!draft)return;
    const select=$('controller-serial'),chosen=draft.controller?.serial||'';
    const found=catalog?.devices||[], unique=new Map();
    for(const d of found) if(d.serial && !unique.has(d.serial))unique.set(d.serial,d);
    let options='<option value="">Automatic</option>';
    for(const [serial,d] of unique) options+=`<option value="${esc(serial)}">${esc(d.model_name||d.name)} · ${esc(serial)}</option>`;
    if(chosen&&!unique.has(chosen))options+=`<option value="${esc(chosen)}">${esc(chosen)} · currently unavailable</option>`;
    if(select.innerHTML!==options)select.innerHTML=options;select.value=chosen;
    if(catalog?.models)$('supported-models').innerHTML=catalog.models.map(m=>`<li><div><strong>${esc(m.name||m.model_name)}</strong><span>0cf2:${usbHex(m.product_id)} · ${m.lighting_supported?'Cooling + lighting':'Cooling · lighting unchanged'}</span></div><span class="verification-badge${m.hardware_tested?' tested':''}">${m.hardware_tested?'Tested':'Untested'}</span></li>`).join('');
  }
  async function discoverDevices(){
    if(discovering)return;discovering=true;$('refresh-devices').disabled=true;$('discovery-status').textContent='Looking for controllers…';
    try{const data=await request('/api/devices');if(!Array.isArray(data.devices)||!Array.isArray(data.models))throw new Error('The service did not return a controller catalog.');catalog=data;renderDevices();if(draft)renderEditor();$('discovery-status').textContent=data.demo?'Discovery is paused in preview mode.':data.devices.length?`${data.devices.length} controller${data.devices.length===1?'':'s'} found.`:'No controller found. Check USB access.';}
    catch(e){$('discovery-status').textContent='Discovery unavailable. '+e.message;}
    finally{discovering=false;$('refresh-devices').disabled=false;}
  }
  $('refresh-devices').addEventListener('click',discoverDevices);
  $('controller-serial').addEventListener('change',event=>{draft.controller={...(draft.controller||{allow_untested:false}),serial:event.target.value||null};dirtyChanged();renderEditor();});
  $('allow-untested').addEventListener('change',event=>{draft.controller={...(draft.controller||{serial:null}),allow_untested:event.target.checked};dirtyChanged();renderEditor();});
  function renderApplyStatus() {
    if (awaitingApply===null) return;
    const el=$('apply-status');el.hidden=false;el.dataset.state='pending';
    if(state.controller?.error && state.controller?.connected){el.textContent='Profile needs attention · '+state.controller.error;el.dataset.state='error';}
    else if(state.demo){el.textContent='Saved in preview mode';awaitingApply=null;}
    else if(!failure && state.controller?.connected && state.applied_revision>=awaitingApply){el.textContent=state.controller.lighting_supported?'Profile applied':'Cooling applied · lighting unchanged';el.dataset.state='applied';awaitingApply=null;}
    else el.textContent=failure || !state.controller?.connected ? 'Saved · waiting for controller' : 'Saved · applying to controller…';
    if(awaitingApply===null) setTimeout(()=>{if(awaitingApply===null)el.hidden=true;},6000);
  }
  function feedback(message,error=false){$('settings-feedback').textContent=message;$('settings-feedback').classList.toggle('error',error);}
  function download(name,value){const url=URL.createObjectURL(new Blob([JSON.stringify(value,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
  function validateImported(c){
    if(!c || !Array.isArray(c.channels) || c.channels.length!==4 || ![1000,2000,3000,4000,5000].includes(c.poll_ms)) throw new Error('Use a W-Connect profile with four banks and a valid control interval.');
    for(const bank of c.channels){
      if(typeof bank.name!=='string'||!bank.name.trim()||[...bank.name].length>40||/[\u0000-\u001f\u007f]/.test(bank.name))throw new Error('Bank names need 1–40 printable characters.');
      if(![1,2,3,4,5,6].includes(bank.fan_count)||!['curve','fixed'].includes(bank.mode)||!['cpu','gpu','max'].includes(bank.source)||!Number.isInteger(bank.fixed_duty)||bank.fixed_duty<40||bank.fixed_duty>100)throw new Error('The profile contains invalid bank settings.');
      const points=bank.curve;
      if(!Array.isArray(points)||points.length<2||points.length>10||points.some((p,i)=>!finite(p.temp)||!finite(p.duty)||p.temp<20||p.temp>85||p.duty<40||p.duty>100||(i>0&&(p.temp<=points[i-1].temp||p.duty<points[i-1].duty)))||points.at(-1).duty!==100)throw new Error('Curves must rise safely from 40% and reach 100% by 85°C.');
      const l=bank.lighting;if(!l||!['preserve','static','breathing','rainbow','off'].includes(l.mode)||!/^#[0-9a-f]{6}$/i.test(l.color)||!Number.isInteger(l.brightness)||l.brightness<0||l.brightness>100)throw new Error('The profile contains invalid lighting settings.');
    }
    // Select only supported fields; never pass extra imported data to hardware.
    return {controller:clone(draft?.controller || {allow_untested:false,serial:null}),poll_ms:c.poll_ms,channels:c.channels.map(b=>({name:b.name,fan_count:b.fan_count,mode:b.mode,source:b.source,fixed_duty:b.fixed_duty,curve:b.curve.map(p=>({temp:p.temp,duty:p.duty})),lighting:{mode:b.lighting.mode,color:b.lighting.color,brightness:b.lighting.brightness}}))};
  }
  for(const [id,key] of [['theme-preference','theme'],['motion-preference','motion']]) $(id).addEventListener('change',event=>{preferences[key]=event.target.value;applyPreferences();try{localStorage.setItem('w-connect-preferences',JSON.stringify(preferences));}catch{toast('Preference applied for this session; browser storage is unavailable.',true);}});
  $('poll-interval').addEventListener('change',event=>{draft.poll_ms=Number(event.target.value);dirtyChanged();});
  $('export-profile').addEventListener('click',()=>{download('w-connect-profile.json',{format:'w-connect-profile',version:1,config:draft});feedback('Profile exported.');});
  $('import-profile').addEventListener('click',()=>$('profile-file').click());
  $('profile-file').addEventListener('change',async event=>{const file=event.target.files[0];if(!file)return;try{if(file.size>65536)throw new Error('Choose a profile smaller than 64 KB.');const data=JSON.parse(await file.text());if(data.format!=='w-connect-profile'||data.version!==1)throw new Error('This is not a supported W-Connect profile.');draft=validateImported(data.config);dirtyChanged();renderEditor();renderTelemetry();feedback('Imported as a draft. Save to apply.');}catch(error){feedback(error.message,true);}finally{event.target.value='';}});
  $('export-diagnostics').addEventListener('click',()=>{download('w-connect-diagnostics.json',{version:state.version,platform:state.platform,controller:state.controller,sensors:state.sensors,channels:state.channels,events:state.events,uptime_secs:state.uptime_secs});toast('Diagnostics exported.');});
  $('color-hex').addEventListener('change',event=>{const value=event.target.value.trim();if(!/^#[0-9a-f]{6}$/i.test(value)){event.target.setAttribute('aria-invalid','true');toast('Enter a color as #RRGGBB, for example #67C8DF.',true);return;}event.target.removeAttribute('aria-invalid');channel().lighting.color=value.toLowerCase();dirtyChanged();renderLighting();});
  $('undo').addEventListener('click',()=>{if(busy||!undoStack.length)return;draft=undoStack.pop();dirtyChanged(false);renderEditor();renderTelemetry();});
  $('retry-connection').addEventListener('click',()=>{poll();});
  document.addEventListener('keydown',event=>{if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='s'){event.preventDefault();save();}if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='z'&&!event.target.matches('input,textarea')){event.preventDefault();$('undo').click();}});
  document.addEventListener('click',event=>{if(!event.target.closest('#status-control'))$('status-control').open=false;});
  document.addEventListener('keydown',event=>{if(event.key==='Escape')$('status-control').open=false;});
  window.addEventListener('beforeunload',event=>{if(dirty){event.preventDefault();event.returnValue='';}});
  window.addEventListener('resize',()=>{if(draft && !dragging)renderGraph();});
  document.addEventListener('visibilitychange',()=>{if(!document.hidden)poll();});
  if(window.ResizeObserver)new ResizeObserver(()=>{if(view==='cooling')renderGraph();}).observe($('curve-chart'));
  poll();
})();
