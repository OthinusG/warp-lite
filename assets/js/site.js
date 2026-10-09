(function () {
  var root = document.documentElement;

  var themeToggle = document.getElementById('theme-toggle');
  if (themeToggle) {
    themeToggle.addEventListener('click', function () {
      var next = root.classList.contains('dark') ? 'light' : 'dark';
      root.classList.toggle('dark', next === 'dark');
      root.style.colorScheme = next;
      try { localStorage.setItem('warpai-theme', next); } catch (e) {}
    });
  }

  var menuToggle = document.getElementById('menu-toggle');
  var mobileNav = document.getElementById('mobile-nav');
  if (menuToggle && mobileNav) {
    var setOpen = function (open) {
      mobileNav.hidden = !open;
      menuToggle.setAttribute('aria-expanded', String(open));
    };
    menuToggle.addEventListener('click', function () {
      setOpen(mobileNav.hidden);
    });
    mobileNav.addEventListener('click', function (event) {
      if (event.target.closest('a')) setOpen(false);
    });
    document.addEventListener('keydown', function (event) {
      if (event.key === 'Escape' && !mobileNav.hidden) setOpen(false);
    });
    window.addEventListener('resize', function () {
      if (window.innerWidth > 800 && !mobileNav.hidden) setOpen(false);
    });
  }
})();
