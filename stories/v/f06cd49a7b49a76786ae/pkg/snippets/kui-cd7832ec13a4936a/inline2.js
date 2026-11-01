
        export function listenForEditorText() {
            const controller = new AbortController();
            const options = { capture: true, signal: controller.signal };
            function attach(input) {
                let composing = false;
                input.addEventListener('compositionstart', () => { composing = true; }, options);
                input.addEventListener('compositionend', () => { composing = false; }, options);
                input.addEventListener('blur', () => { composing = false; }, options);
                input.addEventListener('keydown', event => {
                    if (input.readOnly) return;
                    const shortcut = event.metaKey || (event.ctrlKey && !event.getModifierState('AltGraph'));
                    const textKey = [...event.key].length === 1 || event.key === 'Dead';
                    if (composing || event.isComposing || event.keyCode === 229 || (!shortcut && textKey)) {
                        // Preserve the default action that drives the IME and regular typing.
                        event.stopImmediatePropagation();
                    }
                }, options);
            }
            if (document.activeElement instanceof HTMLTextAreaElement) {
                attach(document.activeElement);
            } else {
                // GPUI may assign editor focus just before it focuses the DOM mirror.
                const onFocus = event => {
                    if (!(event.target instanceof HTMLTextAreaElement)) return;
                    document.removeEventListener('focusin', onFocus, true);
                    attach(event.target);
                };
                document.addEventListener('focusin', onFocus, options);
            }
            return controller;
        }
        export function stopListeningForEditorText(controller) { controller.abort(); }
    