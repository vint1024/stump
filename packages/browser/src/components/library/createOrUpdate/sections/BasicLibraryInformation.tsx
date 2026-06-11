import { Button, Input, Label, Text, TextArea } from '@stump/components'
import { UserPermission } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { Folder, FolderPlus, X } from 'lucide-react'
import { Suspense, useState } from 'react'
import { useFormContext, useFormState, useWatch } from 'react-hook-form'

import DirectoryPickerModal from '@/components/DirectoryPickerModal'
import TagSelect from '@/components/TagSelect'
import { useAppContext } from '@/context'
import { useLibraryContextSafe } from '@/scenes/library/context'

import { CreateOrUpdateLibrarySchema } from '../schema'

const LOCALE_KEY = 'createOrUpdateLibraryForm'
const getKey = (key: string) => `${LOCALE_KEY}.fields.${key}`

type Props = {
	onSetShowDirectoryPicker: (value: boolean) => void
}

export default function BasicLibraryInformation({ onSetShowDirectoryPicker }: Props) {
	const form = useFormContext<CreateOrUpdateLibrarySchema>()
	const ctx = useLibraryContextSafe()
	const { checkPermission } = useAppContext()

	const isCreatingLibrary = !ctx?.library
	const tags = useWatch({ control: form.control, name: 'tags' })
	const extraPaths = useWatch({ control: form.control, name: 'extraPaths' }) ?? []
	// Index of the extra path currently picking a directory, if any
	const [pickingExtraPathIndex, setPickingExtraPathIndex] = useState<number | null>(null)

	const { t } = useLocaleContext()
	const { errors } = useFormState({
		control: form.control,
	})

	return (
		<div className="gap-6 flex grow flex-col">
			<div className="gap-y-6 md:flex-row md:gap-x-6 md:gap-y-6 flex flex-col flex-wrap">
				<Input
					variant="primary"
					label={t(getKey('name.label'))}
					description={t(getKey('name.description'))}
					placeholder={t(getKey('name.placeholder'))}
					containerClassName="max-w-full md:max-w-sm"
					required={isCreatingLibrary}
					errorMessage={errors.name?.message}
					data-1p-ignore
					{...form.register('name')}
				/>

				<Input
					variant="primary"
					label={t(getKey('path.label'))}
					description={t(getKey('path.description'))}
					placeholder={t(getKey('path.placeholder'))}
					containerClassName="max-w-full md:max-w-sm"
					rightDecoration={
						checkPermission(UserPermission.FileExplorer) && (
							<Button size="icon" type="button" onClick={() => onSetShowDirectoryPicker(true)}>
								<Folder className="h-4 w-4 text-foreground-muted" />
							</Button>
						)
					}
					required={isCreatingLibrary}
					errorMessage={errors.path?.message}
					{...form.register('path')}
				/>
			</div>

			<div className="gap-y-3 flex flex-col">
				<div>
					<Label>{t(getKey('extraPaths.label'))}</Label>
					<Text size="sm" variant="muted">
						{t(getKey('extraPaths.description'))}
					</Text>
				</div>

				{extraPaths.map((_, index) => (
					<div key={index} className="gap-2 md:max-w-sm flex max-w-full items-center">
						<Input
							variant="primary"
							containerClassName="flex-1"
							errorMessage={errors.extraPaths?.[index]?.message}
							rightDecoration={
								checkPermission(UserPermission.FileExplorer) && (
									<Button size="icon" type="button" onClick={() => setPickingExtraPathIndex(index)}>
										<Folder className="h-4 w-4 text-foreground-muted" />
									</Button>
								)
							}
							{...form.register(`extraPaths.${index}`)}
						/>
						<Button
							size="icon"
							type="button"
							title={t(getKey('extraPaths.removeFolder'))}
							onClick={() =>
								form.setValue(
									'extraPaths',
									extraPaths.filter((_, i) => i !== index),
									{ shouldDirty: true },
								)
							}
						>
							<X className="h-4 w-4 text-foreground-muted" />
						</Button>
					</div>
				))}
				{errors.extraPaths?.message && (
					<Text size="sm" className="text-fill-danger">
						{errors.extraPaths.message}
					</Text>
				)}

				<div>
					<Button
						type="button"
						variant="outline"
						size="sm"
						onClick={() => form.setValue('extraPaths', [...extraPaths, ''], { shouldDirty: true })}
					>
						<FolderPlus className="mr-2 h-4 w-4" />
						{t(getKey('extraPaths.addFolder'))}
					</Button>
				</div>

				{checkPermission(UserPermission.FileExplorer) && (
					<DirectoryPickerModal
						isOpen={pickingExtraPathIndex !== null}
						onClose={() => setPickingExtraPathIndex(null)}
						startingPath={
							pickingExtraPathIndex !== null ? extraPaths[pickingExtraPathIndex] : undefined
						}
						onPathChange={(path) => {
							if (path && pickingExtraPathIndex !== null) {
								form.setValue(`extraPaths.${pickingExtraPathIndex}`, path, {
									shouldDirty: true,
								})
							}
						}}
					/>
				)}
			</div>

			<TextArea
				className="flex"
				variant="primary"
				label={t(getKey('description.label'))}
				description={t(getKey('description.description'))}
				placeholder={t(getKey('description.placeholder'))}
				containerClassName="max-w-full md:max-w-sm lg:max-w-lg"
				{...form.register('description')}
			/>

			<Suspense fallback={null}>
				<TagSelect
					label={t(getKey('tags.label'))}
					description={t(getKey('tags.description'))}
					selected={tags}
					onChange={(value) => form.setValue('tags', value)}
				/>
			</Suspense>
		</div>
	)
}
