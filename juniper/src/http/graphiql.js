function normalizeSubscriptionEndpoint(endpoint, subscriptionEndpoint) {
    if (subscriptionEndpoint) {
        if (subscriptionEndpoint.startsWith('/')) {
            const secure =
                endpoint.includes('https') || location.href.includes('https')
                    ? 's'
                    : ''
            return `ws${secure}://${location.host}${subscriptionEndpoint}`
        } else {
            return subscriptionEndpoint.replace(/^http/, 'ws')
        }
    }
    return null
}
