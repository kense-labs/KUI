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
    let motionObserved = false;
    document.addEventListener('kui:navigated', () => {
      const heading = document.querySelector('article.doc > h1');
      if (heading && getComputedStyle(heading).animationName === 'doc-enter') motionObserved = true;
    });
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
    if (matchMedia('(prefers-reduced-motion: no-preference)').matches && !motionObserved) {
      throw Error('document entrance animation missing after navigation');
    }
    if (!original.querySelector('#status')?.hidden || original.body.hasAttribute('aria-busy')) {
      throw Error('preview loading state did not clear after paint');
    }
    const install = document.querySelector('[data-copy^="cargo add"]')?.closest('figure');
    if (!install?.querySelector('.syn-tag') || !install.querySelector('.syn-keyword')) {
      throw Error('Shell command or flag highlighting missing');
    }
    document.querySelector('[data-view=code]').click();
    const panel = document.querySelector('[data-panel=code]');
    if (panel.hidden || !panel.querySelector('.syn-keyword')) throw Error('example syntax highlighting missing');
    const source = panel.querySelector('[data-item-source]');
    source.open = true;
    await wait(() => source.dataset.loaded === 'yes', 'installed source did not load', 10000);
    const file = source.querySelector('select');
    const code = source.querySelector('pre code');
    if (!code.querySelector('.syn-keyword')) throw Error('installed source syntax highlighting missing');
    const first = code.textContent;
    if (file.options.length < 2) throw Error('expected multiple installed source files');
    file.selectedIndex = 1;
    file.dispatchEvent(new Event('change', { bubbles: true }));
    if (code.textContent === first || !code.querySelector('.syn-keyword')) {
      throw Error('switching installed source files lost syntax highlighting');
    }
    document.querySelector('[data-view=preview]').click();
    report('loading completion, example highlighting and installed source switching: PASS');
    report('PASS');
  } catch (error) {
    report('FAIL: ' + error);
  }
})();
