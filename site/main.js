// Point the download buttons at what the visitor's device can install; no tracking, nothing stored.
const ua = navigator.userAgent;
const phone = /Android|iPhone|iPad|Mobile/.test(ua);
const os = phone ? null : /Windows/.test(ua) ? 'Windows' : /Linux/.test(ua) ? 'Linux' : /Mac/.test(ua) ? 'Mac' : null;
const files = {
  Windows: 'https://github.com/jonathanvouilloz/barnaby/releases/latest/download/Barnaby-windows-setup.exe',
  Linux: 'https://github.com/jonathanvouilloz/barnaby/releases/latest/download/Barnaby-linux-amd64.deb',
  // Apple Silicon only; browsers do not tell an Intel Mac apart, the FAQ does.
  Mac: 'https://github.com/jonathanvouilloz/barnaby/releases/latest/download/Barnaby-macos-arm64.dmg',
};
const main = document.getElementById('download');
const nav = document.getElementById('nav-download');
if (files[os]) {
  main.textContent = `Télécharger pour ${os}`;
  main.href = nav.href = files[os];
} else if (phone) {
  // A phone cannot install it: the main button says so; a second "Télécharger" would contradict it.
  main.textContent = 'À installer sur ton ordinateur';
  nav.hidden = true;
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

// Motion (GSAP), only for visitors who accept it. Without GSAP or with reduced motion the page stays complete and still.
if (window.gsap && window.ScrollTrigger) {
  gsap.registerPlugin(ScrollTrigger);
  gsap.matchMedia().add('(prefers-reduced-motion: no-preference)', () => {
    gsap.defaults({ ease: 'power3.out' });

    // Hero: the owl lands, then the title, the lead and the buttons follow.
    gsap.timeline()
      .from('.hero-owl', { autoAlpha: 0, scale: 0.86, y: 14, duration: 0.8, ease: 'back.out(1.6)' })
      .from('.hero > :not(.hero-owl)', { autoAlpha: 0, y: 18, duration: 0.6, stagger: 0.08 }, '-=0.5');
    // The inline styles now hold the start state; the safety class from <head> can go.
    document.documentElement.classList.remove('anim');

    // Sections rise in as they scroll into view, a few items at a time.
    const items = gsap.utils.toArray('main > .demo, main > section:not(.hero) > :not(.pair, .grid, .steps), .pair > *, .grid > *, .steps > li');
    gsap.set(items, { autoAlpha: 0, y: 28 });
    // What a jump has already left above the screen appears at once; only what is on screen moves.
    const reveal = (batch) => {
      const above = batch.filter((el) => el.getBoundingClientRect().bottom < 0);
      gsap.set(above, { autoAlpha: 1, y: 0 });
      gsap.to(batch.filter((el) => !above.includes(el)), { autoAlpha: 1, y: 0, duration: 0.7, stagger: 0.09, overwrite: true });
    };
    // end 'max': anything already scrolled past (a jump to #questions, a reload mid-page) counts as entered.
    ScrollTrigger.batch(items, { start: 'top 90%', end: 'max', once: true, onEnter: reveal });

    // Questions unfold and fold instead of snapping open.
    const offs = [...document.querySelectorAll('details')].map((d) => {
      const body = d.querySelector('p');
      const done = () => gsap.set(body, { clearProps: 'height,paddingBottom,opacity,visibility,overflow,transform' });
      const toggle = (e) => {
        e.preventDefault();
        if (d.open && !d.classList.contains('closing')) {
          d.classList.add('closing');
          gsap.to(body, {
            height: 0, paddingBottom: 0, autoAlpha: 0, y: -4, overflow: 'hidden', duration: 0.35, ease: 'power2.inOut', overwrite: true,
            onComplete: () => { d.open = false; d.classList.remove('closing'); done(); },
          });
          return;
        }
        const reopening = d.classList.contains('closing');
        d.classList.remove('closing');
        d.open = true;
        // Natural height, padding included: tweening to 'auto' from a zero padding would snap at the end.
        const { height, paddingBottom } = body.style;
        body.style.height = body.style.paddingBottom = '';
        const natural = body.offsetHeight;
        Object.assign(body.style, { height, paddingBottom });
        const to = { height: natural, paddingBottom: 18, autoAlpha: 1, y: 0, overflow: 'hidden', duration: 0.45, overwrite: true, onComplete: done };
        if (reopening) gsap.to(body, to);
        else gsap.fromTo(body, { height: 0, paddingBottom: 0, autoAlpha: 0, y: -6 }, to);
      };
      const summary = d.querySelector('summary');
      summary.addEventListener('click', toggle);
      return () => summary.removeEventListener('click', toggle);
    });
    return () => offs.forEach((off) => off());
  });
}
