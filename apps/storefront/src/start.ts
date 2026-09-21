import { createMiddleware, createStart } from '@tanstack/react-start'

const stagingSearchEnginePolicy = createMiddleware({ type: 'request' }).server(
  async ({ next }) => {
    const result = await next()

    if (process.env.APP_ENV === 'staging') {
      result.response.headers.set(
        'X-Robots-Tag',
        'noindex, nofollow, noarchive, nosnippet',
      )
    }

    return result
  },
)

export const startInstance = createStart(() => ({
  requestMiddleware: [stagingSearchEnginePolicy],
}))
