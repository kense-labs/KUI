
 async function kuiReadFiles(files,limit) {return Promise.all(Array.from(files).map(async f=>[f.name,f.size,new Uint8Array(f.size<=limit?await f.arrayBuffer():0)]))}
 export function kuiChooseFiles(multiple,accept,max) {return new Promise((resolve,reject)=>{const x=document.createElement('input');x.type='file';x.multiple=multiple;x.accept=accept;
 x.oncancel=()=>resolve([]);x.onchange=()=>kuiReadFiles(x.files,max).then(resolve,reject);x.click()})}
 export function kuiWatchFiles(hit,deliver,over,max) {const c=new AbortController(),opts={capture:true,signal:c.signal};
 function has(t){return t&&Array.from(t.types).includes('Files')}
 let armed=false;document.addEventListener('pointerdown',e=>{armed=hit(e.clientX,e.clientY)},opts);
 function consume(files){kuiReadFiles(files,max).then(result=>{if(!c.signal.aborted)deliver(result)},()=>{if(!c.signal.aborted)deliver(null)})}
 let inside=false;function hover(next){if(next!==inside){inside=next;over(next)}}
 document.addEventListener('dragover',e=>{const on=has(e.dataTransfer)&&hit(e.clientX,e.clientY);hover(on);if(on)e.preventDefault()},opts);
 document.addEventListener('dragleave',e=>{if(!e.relatedTarget)hover(false)},opts);
 document.addEventListener('dragend',()=>hover(false),opts);
 document.addEventListener('drop',e=>{hover(false);if(!has(e.dataTransfer)||!hit(e.clientX,e.clientY))return;e.preventDefault();e.stopImmediatePropagation();consume(e.dataTransfer.files)},opts);
 document.addEventListener('paste',e=>{if(!e.clipboardData?.files?.length||!armed)return;e.preventDefault();consume(e.clipboardData.files)},opts);return c}
 export function kuiUnwatchFiles(c){c.abort()}
 