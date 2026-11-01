// Script injected into the main document by the macOS test runner, never shipped.
(async () => {
  const report = message => webkit.messageHandlers.report.postMessage(message);
  const wait = async (predicate, description, timeout = 4000) => {
    const start = performance.now();
    while (!predicate()) {
      if (performance.now() - start > timeout) throw Error(description);
      await new Promise(resolve => setTimeout(resolve, 10));
    }
  };
  try {
    await new Promise(resolve => document.addEventListener('DOMContentLoaded', resolve));
    if (!location.pathname.endsWith('/button/')) throw Error('sidebar replaced the document');
    const frame = () => document.querySelector('[data-preview-slot] iframe');
    const painted = id => frame()?.dataset.renderedStory === id;
    await wait(() => painted('button') && document.documentElement.dataset.client === 'ready',
      'initial GPUI preview failed', 30000);
    const original = frame().contentDocument;
    const check = async (id, action) => {
      const start = performance.now();
      action();
      await wait(() => painted(id) && location.pathname.endsWith('/' + id + '/'), id);
      if (original !== frame().contentDocument) throw Error('GPUI runtime reloaded: ' + id);
      const elapsed = Math.round(performance.now() - start);
      if (elapsed > 4000) throw Error(id + ' blocked for ' + elapsed + 'ms');
      report(id + ': ' + elapsed + 'ms');
    };
    for (const id of ['action-button', 'split-button', 'date-picker', 'reasoning-block', 'popover', 'button']) {
      await check(id, () => document.querySelector('a[data-nav-item][href$="/' + id + '/"]').click());
    }
    await check('popover', () => history.back());
    await check('button', () => history.forward());
    report('PASS');
  } catch (error) {
    report('FAIL: ' + error);
  }
})();
