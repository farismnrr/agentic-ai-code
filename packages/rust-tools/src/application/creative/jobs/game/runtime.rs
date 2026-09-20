use super::super::super::contracts::GameManifest;
use crate::core::error::McpError;

pub(super) fn reviewed_index_html(game: &GameManifest, template: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><link rel=\"stylesheet\" href=\"style.css\"></head><body data-template=\"{}\"><main><canvas id=\"game\" width=\"960\" height=\"540\"></canvas><div id=\"status\"></div></main><script type=\"module\" src=\"game.js\"></script></body></html>",
        html_escape(&game.title),
        template
    )
}

pub(super) fn reviewed_game_js(game: &GameManifest, template: &str) -> Result<String, McpError> {
    let manifest = serde_json::to_string(game)
        .map_err(|_| McpError::Internal("game source manifest encoding failed".into()))?;
    let template = serde_json::to_string(template)
        .map_err(|_| McpError::Internal("game template encoding failed".into()))?;
    let runtime = r#";
const canvas=document.getElementById('game');
const ctx=canvas.getContext('2d');
const status=document.getElementById('status');
const targetScore=10;
const keys=new Set();
const pointer={active:false,x:canvas.width/2,y:canvas.height/2};
const primaryAssetRole=Array.isArray(manifest.asset_roles)?manifest.asset_roles.find(role=>role&&role.asset_id&&role.runtime_path):null;
const pickupImage=primaryAssetRole?new Image():null;
let pickupImageReady=false;
if(pickupImage){pickupImage.onload=()=>{pickupImageReady=true;};pickupImage.src=primaryAssetRole.runtime_path;}
let state;
function spawnPickup(){
  const inset=70;
  state.pickup.x=inset+((state.score*137+211)%(canvas.width-inset*2));
  state.pickup.y=inset+((state.score*83+97)%(canvas.height-inset*2));
}
function restart(){
  state={phase:'core',score:0,lives:3,t:0,invulnerable:0,
    player:{x:canvas.width/2,y:canvas.height/2,r:18},
    pickup:{x:0,y:0,r:18},
    hazards:[
      {x:canvas.width*0.25,y:canvas.height*0.32,r:22,vx:115,vy:82},
      {x:canvas.width*0.72,y:canvas.height*0.68,r:24,vx:-92,vy:108}
    ]};
  spawnPickup();
}
function win(){state.phase='win';}
function lose(){state.phase='lose';}
function overlaps(a,b){const dx=a.x-b.x,dy=a.y-b.y,r=a.r+b.r;return dx*dx+dy*dy<=r*r;}
function gamepadAxes(){
  if(!manifest.inputs.includes('gamepad')||!navigator.getGamepads)return [0,0];
  const pad=Array.from(navigator.getGamepads()).find(Boolean);
  if(!pad)return [0,0];
  const x=Math.abs(pad.axes[0]||0)>0.15?(pad.axes[0]||0):0;
  const y=Math.abs(pad.axes[1]||0)>0.15?(pad.axes[1]||0):0;
  return [x,y];
}
function update(dt){
  if(state.phase!=='core')return;
  state.t+=dt;state.invulnerable=Math.max(0,state.invulnerable-dt);
  let dx=(keys.has('arrowright')||keys.has('d')?1:0)-(keys.has('arrowleft')||keys.has('a')?1:0);
  let dy=(keys.has('arrowdown')||keys.has('s')?1:0)-(keys.has('arrowup')||keys.has('w')?1:0);
  const [gx,gy]=gamepadAxes();dx+=gx;dy+=gy;
  if(pointer.active){const px=pointer.x-state.player.x,py=pointer.y-state.player.y,dist=Math.hypot(px,py);if(dist>6){dx+=px/dist;dy+=py/dist;}}
  const len=Math.hypot(dx,dy)||1;const speed=220;
  state.player.x=Math.max(state.player.r,Math.min(canvas.width-state.player.r,state.player.x+(dx/len)*speed*dt));
  state.player.y=Math.max(state.player.r,Math.min(canvas.height-state.player.r,state.player.y+(dy/len)*speed*dt));
  for(const hazard of state.hazards){
    hazard.x+=hazard.vx*dt;hazard.y+=hazard.vy*dt;
    if(hazard.x<hazard.r||hazard.x>canvas.width-hazard.r){hazard.vx*=-1;hazard.x=Math.max(hazard.r,Math.min(canvas.width-hazard.r,hazard.x));}
    if(hazard.y<hazard.r||hazard.y>canvas.height-hazard.r){hazard.vy*=-1;hazard.y=Math.max(hazard.r,Math.min(canvas.height-hazard.r,hazard.y));}
  }
  if(overlaps(state.player,state.pickup)){
    state.score+=1;
    if(state.score>=targetScore){win();}else{spawnPickup();}
  }
  if(state.phase==='core'&&state.invulnerable<=0){
    for(const hazard of state.hazards){
      if(overlaps(state.player,hazard)){
        state.lives-=1;state.invulnerable=1;state.player.x=canvas.width/2;state.player.y=canvas.height/2;
        if(state.lives<=0)lose();
        break;
      }
    }
  }
}
function render(){
  ctx.fillStyle='#171a23';ctx.fillRect(0,0,canvas.width,canvas.height);
  ctx.fillStyle='#f7f1df';ctx.font='700 24px system-ui';ctx.fillText(manifest.title,24,36);
  ctx.font='16px system-ui';ctx.fillText(`score ${state.score}/${targetScore}   lives ${state.lives}`,24,64);
  if(pickupImage&&pickupImageReady){ctx.drawImage(pickupImage,state.pickup.x-22,state.pickup.y-22,44,44);}else{ctx.fillStyle='#ff7a24';ctx.fillRect(state.pickup.x-18,state.pickup.y-18,36,36);}
  ctx.fillStyle='#ef476f';for(const hazard of state.hazards){ctx.beginPath();ctx.arc(hazard.x,hazard.y,hazard.r,0,Math.PI*2);ctx.fill();}
  ctx.fillStyle=state.invulnerable>0?'#9ad7ff':'#6ee7b7';ctx.beginPath();ctx.arc(state.player.x,state.player.y,state.player.r,0,Math.PI*2);ctx.fill();
  if(state.phase==='win'||state.phase==='lose'){
    ctx.fillStyle='rgba(0,0,0,.68)';ctx.fillRect(0,0,canvas.width,canvas.height);
    ctx.fillStyle='#fff';ctx.font='700 44px system-ui';ctx.textAlign='center';ctx.fillText(state.phase==='win'?'YOU WIN':'TRY AGAIN',canvas.width/2,canvas.height/2);ctx.textAlign='start';
  }
  status.textContent=`${manifest.core_loop} — ${state.phase}`;
}
function pointFromEvent(event){const rect=canvas.getBoundingClientRect();return {x:(event.clientX-rect.left)*canvas.width/rect.width,y:(event.clientY-rect.top)*canvas.height/rect.height};}
addEventListener('keydown',event=>{const key=event.key.toLowerCase();if(['arrowup','arrowdown','arrowleft','arrowright'].includes(key))event.preventDefault();keys.add(key);if(key==='r'||key==='enter')restart();});
addEventListener('keyup',event=>keys.delete(event.key.toLowerCase()));
canvas.addEventListener('pointerdown',event=>{const point=pointFromEvent(event);pointer.active=true;pointer.x=point.x;pointer.y=point.y;canvas.setPointerCapture?.(event.pointerId);});
canvas.addEventListener('pointermove',event=>{if(pointer.active){const point=pointFromEvent(event);pointer.x=point.x;pointer.y=point.y;}});
canvas.addEventListener('pointerup',()=>{pointer.active=false;});
canvas.addEventListener('pointercancel',()=>{pointer.active=false;});
restart();render();
let last=performance.now();
function frame(now){const dt=Math.min((now-last)/1000,0.05);last=now;update(dt);render();requestAnimationFrame(frame);}
requestAnimationFrame(frame);
export{manifest,state,restart,win,lose,update,render,template,targetScore};
"#;
    let mut script = String::with_capacity(manifest.len() + template.len() + runtime.len() + 128);
    script.push_str(
        "// Reviewed Masih Awam browser-game scaffold; source stays editable.\nconst manifest=",
    );
    script.push_str(&manifest);
    script.push_str(";\nconst template=");
    script.push_str(&template);
    script.push_str(runtime);
    Ok(script)
}

pub(super) fn reviewed_css() -> &'static str {
    "html,body{margin:0;min-height:100%;background:#111;color:#eee;font-family:system-ui}main{display:grid;place-items:center;gap:1rem;padding:1rem}canvas{max-width:100%;height:auto;background:#20242a;border:1px solid #555}"
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
