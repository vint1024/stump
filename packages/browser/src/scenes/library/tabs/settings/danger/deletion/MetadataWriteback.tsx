import { useGraphQLMutation } from '@stump/client'
import { Button, CheckBox, ConfirmationModal, Heading, Text } from '@stump/components'
import { graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useState } from 'react'
import { toast } from 'sonner'

import { useLibraryManagement } from '../../context'

const LOCALE_BASE = 'librarySettingsScene.danger-zone/delete.sections.metadataWriteback'

const writeMutation = graphql(`
	mutation LibraryMetadataWriteback($id: ID!, $backup: Boolean!) {
		writeLibraryMetadataToFiles(id: $id, backup: $backup)
	}
`)

const cleanBackupsMutation = graphql(`
	mutation LibraryMetadataWritebackCleanBackups($id: ID!) {
		cleanMetadataBackups(id: $id)
	}
`)

/**
 * Library-level metadata writeback: a background job embeds the metadata
 * stored in Stump into every epub file of the library, plus a cleaner for the
 * `.bak` copies the job can leave behind
 */
export default function MetadataWriteback() {
	const {
		library: { id },
	} = useLibraryManagement()
	const { t } = useLocaleContext()

	const [backup, setBackup] = useState(false)
	const [showConfirmation, setShowConfirmation] = useState(false)

	const { mutate: writeAll, isPending: isWriting } = useGraphQLMutation(writeMutation, {
		onSuccess: () => {
			toast.success(t(`${LOCALE_BASE}.toasts.started`))
			setShowConfirmation(false)
		},
		onError: (error) => {
			console.error('Failed to start metadata writeback', error)
			toast.error(t(`${LOCALE_BASE}.toasts.startFailed`))
		},
	})

	const { mutate: cleanBackups, isPending: isCleaning } = useGraphQLMutation(cleanBackupsMutation, {
		onSuccess: ({ cleanMetadataBackups: removed }) => {
			toast.success(t(`${LOCALE_BASE}.toasts.cleaned`, { removed }))
		},
		onError: (error) => {
			console.error('Failed to clean backups', error)
			toast.error(t(`${LOCALE_BASE}.toasts.cleanFailed`))
		},
	})

	return (
		<div className="gap-y-4 flex flex-col">
			<div>
				<Heading size="sm">{t(`${LOCALE_BASE}.heading`)}</Heading>
				<Text size="sm" variant="muted">
					{t(`${LOCALE_BASE}.description`)}
				</Text>
			</div>

			<CheckBox
				variant="primary"
				label={t(`${LOCALE_BASE}.keepBackup`)}
				checked={backup}
				onClick={() => setBackup((value) => !value)}
			/>

			<div className="gap-2 flex items-center">
				<Button
					type="button"
					variant="outline"
					disabled={isWriting}
					onClick={() => setShowConfirmation(true)}
				>
					{t(`${LOCALE_BASE}.writeAll`)}
				</Button>
				<Button
					type="button"
					variant="outline"
					disabled={isCleaning}
					onClick={() => cleanBackups({ id })}
				>
					{t(`${LOCALE_BASE}.cleanBackups`)}
				</Button>
			</div>

			<ConfirmationModal
				title={t(`${LOCALE_BASE}.confirm.title`)}
				description={t(
					backup ? `${LOCALE_BASE}.confirm.withBackup` : `${LOCALE_BASE}.confirm.withoutBackup`,
				)}
				confirmText={t(`${LOCALE_BASE}.confirm.confirm`)}
				isOpen={showConfirmation}
				onClose={() => setShowConfirmation(false)}
				onConfirm={() => writeAll({ id, backup })}
				trigger={null}
			/>
		</div>
	)
}
