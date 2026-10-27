'use strict';
(()=>{
 let pending=null;
 const native=matchMedia('(display-mode: standalone)').matches||navigator.standalone===true;
 const style=document.createElement('style');style.textContent='.schema-install{width:min(440px,calc(100vw - 24px));max-height:85dvh;overflow:auto;padding:24px;border:1px solid #40534b;border-radius:14px;background:#17201d;color:#e5eaee;font:14px/1.6 system-ui}.schema-install::backdrop{background:#0009}.schema-install button{padding:12px 16px;border-radius:7px;border:1px solid #40534b;background:#24392f;color:#e5eaee;cursor:pointer}.schema-install h2{font-size:25px;margin-top:0}.schema-install .install-close{float:right}.schema-install p{margin:14px 0}';document.head.append(style);
 const dialog=document.createElement('dialog');dialog.className='schema-install';dialog.innerHTML='<button class="install-close" aria-label="Close installation help">×</button><h2>Your pocket controller.</h2><p id="install-context"></p><button id="install-confirm" hidden>Install schémaboy</button><p><strong>iPhone / iPad</strong><br>Open this controller in your browser. Choose Share → Add to Home Screen → Open as Web App, if offered.</p><p><strong>Android</strong><br>Choose Install app or Add to Home screen in the browser menu. Availability depends on your browser and connection.</p><p>The app controls this engine. Keep your phone connected to its network; the Rust engine continues to run on your computer or lighting controller.</p><a href="/mobile?install=1">Open phone controller ↗</a>';document.body.append(dialog);
 const context=dialog.querySelector('#install-context'),confirm=dialog.querySelector('#install-confirm');
 function update(){context.textContent=native?'schémaboy is already open as an app.':!isSecureContext?'This LAN address uses HTTP. You can add a home-screen shortcut if your browser offers it. Full app installation needs a trusted HTTPS connection to the same controller.':pending?'Install schémaboy for a dedicated window and home-screen icon.':'Use your browser’s install or home-screen menu to keep schémaboy close.';confirm.hidden=!pending||native;}
 function open(){update();if(!dialog.open)dialog.showModal();}
 dialog.querySelector('.install-close').onclick=()=>dialog.close();
 confirm.onclick=async()=>{if(!pending)return;const request=pending;pending=null;try{await request.prompt();await request.userChoice;}finally{update();}};
 const targets=[...document.querySelectorAll('[data-install]')];const host=document.querySelector('.app-menu-panel')||document.querySelector('.header-actions');
 if(host){const button=document.createElement('button');button.type='button';button.textContent='Install app';button.dataset.install='';host.append(button);targets.push(button);}
 targets.forEach(button=>button.addEventListener('click',event=>{event.preventDefault();open();}));
 addEventListener('beforeinstallprompt',event=>{event.preventDefault();pending=event;update();});addEventListener('appinstalled',()=>{pending=null;update();dialog.close();});
 if(isSecureContext&&'serviceWorker' in navigator&&location.pathname.startsWith('/mobile'))navigator.serviceWorker.register('/mobile-worker.js',{scope:'/mobile',updateViaCache:'none'}).catch(()=>{});
 if(new URLSearchParams(location.search).get('install')==='1')open();
})();
