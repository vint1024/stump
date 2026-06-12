import { useSuspenseGraphQL } from '@stump/client'
import { Statistic } from '@stump/components'
import { graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { Api } from '@stump/sdk'
import { QueryClient } from '@tanstack/react-query'
import { useMemo } from 'react'

const query = graphql(`
	query UserStats {
		userCount
		topReaders(take: 1) {
			id
			username
			finishedReadingSessionsCount
		}
		activeReadingSessionCount
		finishedReadingSessionCount
	}
`)

export const prefetchUserStats = async (sdk: Api, client: QueryClient) =>
	client.prefetchQuery({
		queryKey: ['userStats'],
		queryFn: async () => {
			const data = await sdk.execute(query)
			return data
		},
	})

export default function UsersStats() {
	const { t } = useLocaleContext()
	const { data } = useSuspenseGraphQL(query, ['userStats'])

	const [powerReader] = useMemo(() => data.topReaders, [data.topReaders])

	return (
		<div className="gap-4 pb-8 scrollbar-hide flex items-center divide-x divide-edge-subtle overflow-x-scroll">
			<Statistic className="pr-5 md:pr-10 shrink-0">
				<Statistic.Label>{t(getKey('users'))}</Statistic.Label>
				<Statistic.CountUpNumber value={data.userCount} />
			</Statistic>

			<Statistic className="px-5 md:px-10 shrink-0">
				<Statistic.Label>{t(getKey('booksCompleted'))}</Statistic.Label>
				<Statistic.CountUpNumber value={data.finishedReadingSessionCount} />
			</Statistic>

			<Statistic className="px-5 md:px-10 shrink-0">
				<Statistic.Label>{t(getKey('booksInProgress'))}</Statistic.Label>
				<Statistic.CountUpNumber value={data.activeReadingSessionCount} />
			</Statistic>

			{!!powerReader && (
				<Statistic className="pl-5 md:pl-10 shrink-0">
					<Statistic.Label>{t(getKey('powerReader'))}</Statistic.Label>
					<Statistic.StringValue>
						{powerReader.username}{' '}
						<span className="text-sm font-normal">
							(
							{t(getKey('booksCount'), {
								count: powerReader.finishedReadingSessionsCount,
							})}
							)
						</span>
					</Statistic.StringValue>
				</Statistic>
			)}
		</div>
	)
}

const LOCALE_BASE = 'scenes.settings.server.users.UsersStats'
const getKey = (key: string) => `${LOCALE_BASE}.${key}`
