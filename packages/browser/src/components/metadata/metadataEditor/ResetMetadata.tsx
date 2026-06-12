import {
	Alert,
	AlertDescription,
	AlertTitle,
	Button,
	cn,
	ConfirmationModal,
	RadioGroup,
} from '@stump/components'
import { MetadataResetImpact } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { AlertTriangle } from 'lucide-react'
import { useState } from 'react'

type Props = {
	onConfirmReset: (impact: MetadataResetImpact) => void
	isDisabled?: boolean
}

export default function ResetMetadata({ onConfirmReset, isDisabled }: Props) {
	const { t } = useLocaleContext()
	const [showConfirmation, setShowConfirmation] = useState(false)

	const [impact, setImpact] = useState<MetadataResetImpact>(MetadataResetImpact.Series)

	const handleChange = (value: string) => {
		if (isImpact(value)) {
			setImpact(value)
		}
	}

	return (
		<div>
			<Button variant="danger" disabled={isDisabled} onClick={() => setShowConfirmation(true)}>
				{t('components.metadata.metadataEditor.ResetMetadata.deleteMetadata')}
			</Button>

			<ConfirmationModal
				title={t('components.metadata.metadataEditor.ResetMetadata.deleteMetadata')}
				description={t('components.metadata.metadataEditor.ResetMetadata.selectImpact')}
				isOpen={showConfirmation}
				confirmVariant="danger"
				confirmText={t('components.metadata.metadataEditor.ResetMetadata.confirmDelete')}
				onConfirm={() => {
					onConfirmReset(impact)
					setShowConfirmation(false)
				}}
				onClose={() => setShowConfirmation(false)}
				size="md"
			>
				<RadioGroup
					value={impact}
					onValueChange={handleChange}
					className="divide gap-0 space-y-0 rounded-xl divide-y divide-edge overflow-hidden border border-edge"
				>
					<RadioGroup.CardItem
						label={t('components.metadata.metadataEditor.ResetMetadata.seriesLabel')}
						value="SERIES"
						description={t('components.metadata.metadataEditor.ResetMetadata.seriesDescription')}
						className={cn('rounded-b-none border-0 bg-background hover:bg-background-surface/50', {
							'bg-background-surface/70 hover:bg-background-surface/70':
								impact === MetadataResetImpact.Series,
						})}
					/>

					<RadioGroup.CardItem
						label={t('components.metadata.metadataEditor.ResetMetadata.booksLabel')}
						value="BOOKS"
						description={t('components.metadata.metadataEditor.ResetMetadata.booksDescription')}
						className={cn('rounded-t-none border-0 bg-background hover:bg-background-surface/50', {
							'bg-background-surface/70 hover:bg-background-surface/70':
								impact === MetadataResetImpact.Books,
						})}
					/>

					<RadioGroup.CardItem
						label={t('components.metadata.metadataEditor.ResetMetadata.everythingLabel')}
						value="EVERYTHING"
						description={t(
							'components.metadata.metadataEditor.ResetMetadata.everythingDescription',
						)}
						className={cn('rounded-t-none border-0 bg-background hover:bg-background-surface/50', {
							'bg-background-surface/70 hover:bg-background-surface/70':
								impact === MetadataResetImpact.Everything,
						})}
					/>
				</RadioGroup>

				<Alert variant="warning">
					<AlertTriangle />
					<AlertTitle>
						{t('components.metadata.metadataEditor.ResetMetadata.cannotBeUndone')}
					</AlertTitle>
					<AlertDescription>
						{t('components.metadata.metadataEditor.ResetMetadata.permanentWarning')}
					</AlertDescription>
				</Alert>
			</ConfirmationModal>
		</div>
	)
}

const isImpact = (value: string): value is MetadataResetImpact => {
	return Object.values(MetadataResetImpact).includes(value as MetadataResetImpact)
}
