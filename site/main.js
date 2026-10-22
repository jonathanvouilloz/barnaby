// Point the download buttons at what the visitor's device can install; no tracking, nothing stored.
const ua = navigator.userAgent;
const phone = /Android|iPhone|iPad|Mobile/.test(ua);
const os = phone ? null : /Windows/.test(ua) ? 'Windows' : /Linux/.test(ua) ? 'Linux' : /Mac/.test(ua) ? 'Mac' : null;
const files = {
  Windows: 'https://github.com/jonathanvouilloz/barnaby/releases/latest/download/Barnaby-windows-setup.exe',
  Linux: 'https://github.com/jonathanvouilloz/barnaby/releases/latest/download/Barnaby-linux-amd64.deb',
};
const main = document.getElementById('download');
const nav = document.getElementById('nav-download');
if (files[os]) {
  main.textContent = `Télécharger pour ${os}`;
  main.href = nav.href = files[os];
} else {
  // Mac and phones: the main button says why; a second "Télécharger" would contradict it.
  nav.hidden = true;
}
if (os === 'Mac') {
  main.textContent = 'Pas encore sur Mac';
  main.href = '#mac';
  main.addEventListener('click', () => { document.getElementById('mac').open = true; });
} else if (phone) {
  main.textContent = 'À installer sur ton ordinateur';
}

const still = matchMedia('(prefers-reduced-motion: reduce)').matches || !('IntersectionObserver' in window);

// The animated owl: a transparent WebM (VP9 alpha). Safari drops the alpha, so it keeps the still frame.
// shortcut: Safari sees no animation; upgrade with an HEVC-alpha .mov made on a Mac.
const safari = /Safari/.test(ua) && !/Chrome|Chromium|Edg|Firefox/.test(ua);
if (!still && !safari) {
  const img = document.getElementById('hero-owl');
  const video = Object.assign(document.createElement('video'), {
    src: 'img/owl-loop.webm', muted: true, loop: true, autoplay: true, playsInline: true, poster: img.src,
  });
  video.setAttribute('aria-label', img.alt);
  video.width = img.width;
  video.height = img.height;
  video.addEventListener('canplay', () => img.replaceWith(video), { once: true });
  video.load();
}

// The owl pops up next to a few headings, once, when they scroll into view.
if (!still) {
  const peek = new IntersectionObserver((entries) => {
    for (const e of entries) if (e.isIntersecting) { e.target.classList.add('seen'); peek.unobserve(e.target); }
  }, { threshold: 0.5 });
  document.querySelectorAll('.owl-peek').forEach((el) => { el.classList.add('hiding'); peek.observe(el); });
}

// Types the demo sentence once, when it scrolls into view; the full text stays readable without motion.
const typed = document.getElementById('typed');
const demo = document.querySelector('.demo');
const owl = document.getElementById('bubble-owl');
if (!still) {
  const words = typed.textContent.split(' ');
  new IntersectionObserver((entries, obs) => {
    if (!entries[0].isIntersecting) return;
    obs.disconnect();
    demo.classList.add('listening');
    owl.src = 'img/listening.png';
    typed.textContent = '';
    let i = 0;
    const step = () => {
      typed.textContent = words.slice(0, ++i).join(' ');
      if (i < words.length) return setTimeout(step, 140);
      setTimeout(() => {
        demo.classList.remove('listening');
        owl.src = 'img/success.png';
      }, 500);
    };
    setTimeout(step, 600);
  }, { threshold: 0.6 }).observe(demo);
}
