// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://uzumaki.run',
  integrations: [
    starlight({
      title: 'Uzumaki',
      logo: {
        light: './src/assets/logo.svg',
        dark: './src/assets/logo.svg',
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/golok727/uzumaki',
        },
        {
          icon: 'x.com',
          label: 'X',
          href: 'https://x.com/golok727',
        },
      ],
      components: {
        Footer: './src/components/Footer.astro',
      },
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Installation', slug: 'guides/installation' },
            { label: 'Quick Start', slug: 'guides/quick-start' },
            { label: 'Configuration', slug: 'guides/configuration' },
            { label: 'Building Your App', slug: 'guides/building' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Elements', slug: 'reference/elements' },
            { label: 'Props', slug: 'reference/props' },
            { label: 'Events', slug: 'reference/events' },
            { label: 'Window', slug: 'reference/window' },
            { label: 'Paths and Resources', slug: 'reference/paths' },
            { label: 'Runtime API', slug: 'reference/runtime-api' },
          ],
        },
      ],
      head: [
        {
          tag: 'link',
          attrs: {
            rel: 'preconnect',
            href: 'https://fonts.googleapis.com',
          },
        },
        {
          tag: 'link',
          attrs: {
            rel: 'preconnect',
            href: 'https://fonts.gstatic.com',
            crossorigin: '',
          },
        },
        {
          tag: 'link',
          attrs: {
            rel: 'stylesheet',
            href: 'https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700&family=Geist+Mono:wght@400;500;600;700&display=swap',
          },
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:image',
            content: 'https://uzumaki.run/social_preview.png',
          },
        },
        {
          tag: 'meta',
          attrs: {
            property: 'og:image:alt',
            content: 'Uzumaki',
          },
        },
        {
          tag: 'meta',
          attrs: {
            name: 'twitter:card',
            content: 'summary_large_image',
          },
        },
        {
          tag: 'meta',
          attrs: {
            name: 'twitter:image',
            content: 'https://uzumaki.run/social_preview.png',
          },
        },
      ],
    }),
  ],
});
