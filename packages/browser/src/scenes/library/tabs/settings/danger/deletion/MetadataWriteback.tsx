import { useGraphQLMutation } from '@stump/client'
import { Button, CheckBox, ConfirmationModal, Heading, Text } from '@stump/components'
import { graphql } from '@stump/graphql'
import { useState } from 'react'
import { toast } from 'sonner'

import { useLibraryManagement } from '../../context'

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

	const [backup, setBackup] = useState(false)
	const [showConfirmation, setShowConfirmation] = useState(false)

	const { mutate: writeAll, isPending: isWriting } = useGraphQLMutation(writeMutation, {
		onSuccess: () => {
			toast.success('Metadata writeback job started')
			setShowConfirmation(false)
		},
		onError: (error) => {
			console.error('Failed to start metadata writeback', error)
			toast.error('Failed to start metadata writeback')
		},
	})

	const { mutate: cleanBackups, isPending: isCleaning } = useGraphQLMutation(cleanBackupsMutation, {
		onSuccess: ({ cleanMetadataBackups: removed }) => {
			toast.success(`Removed ${removed} backup file${removed === 1 ? '' : 's'}`)
		},
		onError: (error) => {
			console.error('Failed to clean backups', error)
			toast.error('Failed to clean backups')
		},
	})

	return (
		<div className="gap-y-4 flex flex-col">
			<div>
				<Heading size="sm">Write metadata to files</Heading>
				<Text size="sm" variant="muted">
					Embed the metadata stored in Stump into every epub file of this library (OPF), so it
					travels with the files. Archives are rewritten atomically; this modifies your files on
					disk
				</Text>
			</div>

			<CheckBox
				variant="primary"
				label="Keep a .bak copy of each original file"
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
					Write metadata to all files
				</Button>
				<Button
					type="button"
					variant="outline"
					disabled={isCleaning}
					onClick={() => cleanBackups({ id })}
				>
					Delete .bak backups
				</Button>
			</div>

			<ConfirmationModal
				title="Write metadata into every epub?"
				description={
					backup
						? 'Every epub in this library will be rewritten with the metadata stored in Stump. Originals are kept as .bak files next to the books.'
						: 'Every epub in this library will be rewritten with the metadata stored in Stump. No backups will be kept.'
				}
				confirmText="Start writeback"
				isOpen={showConfirmation}
				onClose={() => setShowConfirmation(false)}
				onConfirm={() => writeAll({ id, backup })}
				trigger={null}
			/>
		</div>
	)
}
