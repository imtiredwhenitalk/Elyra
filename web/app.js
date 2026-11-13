/* Elyra marketing site — small, dependency-free enhancements. */
(() => {
  const root = document.documentElement;
  root.classList.add("js");

  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  /* ------------------------------------------------------------ reveal */
  const reveals = Array.from(document.querySelectorAll(".reveal"));

  const revealAll = () => {
    for (const element of reveals) element.classList.add("is-visible");
  };

  if (reduceMotion.matches || !("IntersectionObserver" in window)) {
    revealAll();
  } else {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.add("is-visible");
          observer.unobserve(entry.target);
        }
      },
      { rootMargin: "0px 0px -6% 0px", threshold: 0.12 },
    );

    for (const element of reveals) observer.observe(element);

    // Anything already on screen should appear without waiting for a scroll.
    window.requestAnimationFrame(() => {
      const viewport = window.innerHeight || 800;
      for (const element of reveals) {
        if (element.getBoundingClientRect().top < viewport * 0.94) {
          element.classList.add("is-visible");
        }
      }
    });
  }

  /* ------------------------------------------------------------ header */
  const header = document.querySelector("[data-header]");
  if (header) {
    let ticking = false;

    const syncHeader = () => {
      header.classList.toggle("scrolled", window.scrollY > 8);
      ticking = false;
    };

    syncHeader();
    window.addEventListener(
      "scroll",
      () => {
        if (ticking) return;
        ticking = true;
        window.requestAnimationFrame(syncHeader);
      },
      { passive: true },
    );
  }

  /* --------------------------------------------------------- mobile nav */
  const menuButton = document.querySelector("[data-menu]");
  const nav = document.querySelector("[data-nav]");

  if (header && menuButton && nav) {
    const setMenu = (open) => {
      header.classList.toggle("nav-open", open);
      menuButton.setAttribute("aria-expanded", String(open));
      menuButton.setAttribute("aria-label", open ? "Close menu" : "Open menu");
    };

    menuButton.addEventListener("click", (event) => {
      event.stopPropagation();
      setMenu(!header.classList.contains("nav-open"));
    });

    nav.addEventListener("click", (event) => {
      if (event.target.closest("a")) setMenu(false);
    });

    document.addEventListener("click", (event) => {
      if (!header.contains(event.target)) setMenu(false);
    });

    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape") setMenu(false);
    });

    const wide = window.matchMedia("(min-width: 861px)");
    const onWide = (event) => {
      if (event.matches) setMenu(false);
    };
    if (typeof wide.addEventListener === "function") {
      wide.addEventListener("change", onWide);
    } else if (typeof wide.addListener === "function") {
      wide.addListener(onWide);
    }
  }

  /* -------------------------------------------------------------- year */
  const year = new Date().getFullYear();
  for (const element of document.querySelectorAll("[data-year]")) {
    element.textContent = String(year);
  }

  /* -------------------------------------------------------------- faq */
  const faq = document.querySelector(".faq-list");
  if (faq) {
    faq.addEventListener("toggle", (event) => {
      const opened = event.target;
      if (!(opened instanceof HTMLDetailsElement) || !opened.open) return;
      for (const details of faq.querySelectorAll("details[open]")) {
        if (details !== opened) details.open = false;
      }
    }, true);
  }

  /* -------------------------------------------------------- download CTA */
  // The primary hero button scrolls to the download section. Once a visitor
  // is there, the direct MSI link is the loudest action on the page.
  const primary = document.querySelector("[data-primary-download]");
  if (primary) {
    primary.addEventListener("click", () => {
      const msi = document.querySelector(".download-primary");
      if (msi) window.setTimeout(() => msi.focus({ preventScroll: true }), 450);
    });
  }
})();
