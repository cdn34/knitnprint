import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/robots.txt')({
  server: {
    handlers: {
      GET: async () => {
        const preventIndexing = process.env.APP_ENV === 'staging'
        const body = preventIndexing
          ? 'User-agent: *\nDisallow: /\n'
          : 'User-agent: *\nAllow: /\n'

        return new Response(body, {
          headers: {
            'cache-control': 'no-store',
            'content-type': 'text/plain; charset=utf-8',
          },
        })
      },
    },
  },
})
