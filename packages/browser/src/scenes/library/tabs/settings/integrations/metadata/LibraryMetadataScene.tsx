import { Suspense } from 'react'

import { PendingMatchesSection } from '@/components/metadata/metadataMatching'

import InitFetchJob from './InitFetchJob'

export default function LibraryMetadataScene() {
	return (
		<div className="gap-y-12 flex flex-col">
			<PendingMatchesSection />
			<Suspense>
				<InitFetchJob />
			</Suspense>
		</div>
	)
}
