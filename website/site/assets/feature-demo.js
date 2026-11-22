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
