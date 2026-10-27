'use strict';
(()=>{try{const mobile=location.pathname.startsWith('/mobile'),choice=new URLSearchParams(location.search).get('view');if(mobile)localStorage.setItem('schema-view','mobile');else if(choice==='studio')localStorage.setItem('schema-view','studio');else if(matchMedia('(max-width:700px)').matches&&localStorage.getItem('schema-view')==='mobile')location.replace('/mobile');}catch{}})();
