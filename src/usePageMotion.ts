import { useRef } from 'react';
import { useGSAP } from '@gsap/react';
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(useGSAP, ScrollTrigger);

export function usePageMotion(page: string) {
  const scope = useRef<HTMLDivElement>(null);
  useGSAP(
    () => {
      const media = gsap.matchMedia();
      media.add('(prefers-reduced-motion: no-preference)', () => {
        gsap.fromTo(
          '.page-heading, .network-panel, .rules-section, .diagnostics-hero, .profiles-panel, .guide-layout',
          { y: 8, opacity: 0 },
          {
            y: 0,
            opacity: 1,
            duration: 0.3,
            stagger: 0.035,
            ease: 'power2.out',
            clearProps: 'transform,opacity',
          },
        );
        if (page === 'guide') {
          gsap.utils.toArray<HTMLElement>('.guide-step').forEach((step) => {
            gsap.fromTo(
              step,
              { y: 12, opacity: 0.65 },
              {
                y: 0,
                opacity: 1,
                ease: 'none',
                scrollTrigger: { trigger: step, start: 'top 95%', end: 'top 70%', scrub: true },
              },
            );
          });
        }
      });
      media.add('(min-width: 1100px) and (prefers-reduced-motion: no-preference)', () => {
        if (page === 'guide') {
          const aside = scope.current?.querySelector<HTMLElement>('.guide-aside');
          const main = scope.current?.querySelector<HTMLElement>('.guide-main');
          if (!aside || !main || aside.offsetHeight >= main.offsetHeight) return;
          ScrollTrigger.create({
            trigger: main,
            start: 'top 32px',
            end: () => `bottom ${32 + aside.offsetHeight}px`,
            pin: aside,
            pinSpacing: false,
            invalidateOnRefresh: true,
          });
        }
      });
      return () => media.revert();
    },
    { scope, dependencies: [page], revertOnUpdate: true },
  );
  return scope;
}
