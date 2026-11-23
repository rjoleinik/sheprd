'use strict';
// A feature page's demo: one recording per window, narrated in each
// language. The window and language buttons swap the clip and keep the
// place; the chapter buttons seek and play.
const demo = document.querySelector('#feature-demo');
if (demo instanceof HTMLVideoElement) {
  const labels = { en: 'English', fr: 'Français' };
  const chosen = { window: demo.dataset.window, lang: 'en' };
  const play = async () => {
    try {
      await demo.play();
    } catch {
      // Native controls remain available when the browser blocks playback.
      demo.focus();
    }
  };
  // Space plays and pauses the demo anywhere on the page, as on video sites,
  // instead of scrolling it: also while the video, a chapter, or a window or
  // language button has the keyboard. Only typing in a field keeps Space, and
  // Enter still presses a focused button. Listening while capturing runs
  // before the browser's own handling, so the video never toggles twice.
  window.addEventListener('keydown', (event) => {
    if (event.key !== ' ' || event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target;
    if (target instanceof HTMLElement
        && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))) return;
    event.preventDefault();
    event.stopPropagation();
    if (demo.paused) play(); else demo.pause();
  }, { capture: true });
  document.querySelectorAll('[data-seek]').forEach((button) => {
    button.addEventListener('click', () => {
      demo.currentTime = Number(button.dataset.seek);
      play();
    });
  });
  const show = () => {
    const at = demo.currentTime;
    const playing = !demo.paused;
    const name = `${demo.dataset.clips}.${chosen.window}.${chosen.lang}`;
    demo.querySelector('source').src = `${name}.mp4`;
    const track = demo.querySelector('track');
    track.src = `${name}.vtt`;
    track.srclang = chosen.lang;
    track.label = labels[chosen.lang];
    demo.load();
    demo.addEventListener('loadedmetadata', () => {
      demo.currentTime = at;
      if (playing) play();
    }, { once: true });
  };
  ['window', 'lang'].forEach((key) => {
    const buttons = document.querySelectorAll(`.demo-switches [data-${key}]`);
    buttons.forEach((button) => {
      button.addEventListener('click', () => {
        chosen[key] = button.dataset[key];
        buttons.forEach((other) => {
          other.setAttribute('aria-pressed', String(other === button));
        });
        show();
      });
    });
  });
}
