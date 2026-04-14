import { QueryClient } from '@tanstack/react-query'

export const serversQueryClient = new QueryClient({
	defaultOptions: {
		queries: {
			retry: true,
			throwOnError: false,
		},
	},
})
