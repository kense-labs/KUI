
        export function compactHost(){try{return (window.frameElement?.ownerDocument.defaultView ?? window).matchMedia('(max-width:639px)').matches}catch{return innerWidth<640}}
        export function storedAppearance(){try{return ({light:0,dark:1})[localStorage.getItem('theme')] ?? 2}catch{return 2}}
        export function accountAppearance(mode){parent.postMessage({type:'kui:account-appearance',mode},location.origin)}
        export function previewStarted() { queueMicrotask(() => window.kuiPreviewStarted?.()); }
        export function reportThemeTarget(x,y,w,h,radius) { window.kuiThemeTarget?.(x,y,w,h,radius); }
        export function horizontalRegion(x,y,w,h,left,right) { window.kuiHorizontalScroll?.(x,y,w,h,left,right); }
        export function editorRegion(x,y,w,h,up,down) { window.kuiEditorScroll?.(x,y,w,h,up,down); }
        export function downloadCsv(name, csv) {
            const url = URL.createObjectURL(new Blob([csv], {type: 'text/csv;charset=utf-8'}));
            const link = document.createElement('a');
            link.href = url;
            link.download = name;
            document.body.append(link);
            link.click();
            link.remove();
            setTimeout(() => URL.revokeObjectURL(url), 30000);
        }
        export function reportFrame(height, dark) {
            window.kuiFramePainted?.(Math.ceil(height), dark);
        }
        export function previewVisible() { return !document.hidden && self.kuiStoryInViewport !== false; }
        export function leavePreview(back) {
            parent.postMessage({type:'kui:story-leave',back},location.origin);
        }
        export function buttonTheme(dark,mode,x,y,w,h) {
            if (window.kuiPointerActivated?.()) return;
            if (window !== parent) parent.postMessage({type:'kui:family-theme',dark,mode,x,y,w,h},location.origin);
        }

    