import { Button, Input, InputGroup, Label, Text, TextArea } from '@stump/components'
import { UserPermission } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { Folder, Plus, X } from 'lucide-react'
import { Suspense } from 'react'
import { useFormContext, useFormState, useWatch } from 'react-hook-form'

import TagSelect from '@/components/TagSelect'
import { useAppContext } from '@/context'
import { useLibraryContextSafe } from '@/scenes/library/context'

import { CreateOrUpdateLibrarySchema } from '../schema'

const LOCALE_KEY = 'createOrUpdateLibraryForm'
const getKey = (key: string) => `${LOCALE_KEY}.fields.${key}`

type Props = {
	onPickDirectory: (field: 'path' | `extraPaths.${number}`) => void
}

export default function BasicLibraryInformation({ onPickDirectory }: Props) {
	const form = useFormContext<CreateOrUpdateLibrarySchema>()
	const ctx = useLibraryContextSafe()
	const { checkPermission } = useAppContext()

	const isCreatingLibrary = !ctx?.library
	const tags = useWatch({ control: form.control, name: 'tags' })
	const extraPaths = useWatch({ control: form.control, name: 'extraPaths' }) ?? []

	const { t } = useLocaleContext()
	const { errors } = useFormState({
		control: form.control,
	})

	return (
		<div className="gap-6 flex grow flex-col">
			<div className="gap-y-6 md:flex-row md:gap-x-6 md:gap-y-6 flex flex-col flex-wrap">
				<Input
					label={t(getKey('name.label'))}
					description={t(getKey('name.description'))}
					placeholder={t(getKey('name.placeholder'))}
					containerClassName="max-w-full md:max-w-sm"
					required={isCreatingLibrary}
					errorMessage={errors.name?.message}
					data-1p-ignore
					{...form.register('name')}
				/>

				<div className="gap-2 md:max-w-sm grid w-full max-w-full items-center">
					<Label htmlFor="path">
						{t(getKey('path.label'))}
						{isCreatingLibrary && <span className="text-destructive"> *</span>}
					</Label>

					<InputGroup>
						<InputGroup.Input
							id="path"
							placeholder={t(getKey('path.placeholder'))}
							required={isCreatingLibrary}
							aria-invalid={!!errors.path?.message}
							{...form.register('path')}
						/>

						{checkPermission(UserPermission.FileExplorer) && (
							<InputGroup.Addon align="inline-end">
								<InputGroup.Button
									type="button"
									variant="ghost"
									size="icon-xs"
									onClick={() => onPickDirectory('path')}
								>
									<Folder className="h-4 w-4 text-muted-foreground" />
								</InputGroup.Button>
							</InputGroup.Addon>
						)}
					</InputGroup>

					{errors.path?.message && (
						<Text variant="danger" size="xs" className="break-all">
							{errors.path.message}
						</Text>
					)}

					{!errors.path?.message && (
						<Text variant="muted" size="sm">
							{t(getKey('path.description'))}
						</Text>
					)}
				</div>
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
						<InputGroup className="flex-1">
							<InputGroup.Input
								placeholder={t(getKey('path.placeholder'))}
								aria-invalid={!!errors.extraPaths?.[index]?.message}
								{...form.register(`extraPaths.${index}`)}
							/>
							{checkPermission(UserPermission.FileExplorer) && (
								<InputGroup.Addon align="inline-end">
									<InputGroup.Button
										type="button"
										variant="ghost"
										size="icon-xs"
										onClick={() => onPickDirectory(`extraPaths.${index}`)}
									>
										<Folder className="h-4 w-4 text-muted-foreground" />
									</InputGroup.Button>
								</InputGroup.Addon>
							)}
						</InputGroup>
						<Button
							type="button"
							variant="ghost"
							size="icon"
							title={t(getKey('extraPaths.removeFolder'))}
							onClick={() =>
								form.setValue(
									'extraPaths',
									extraPaths.filter((_, i) => i !== index),
									{ shouldDirty: true },
								)
							}
						>
							<X className="h-4 w-4 text-muted-foreground" />
						</Button>
					</div>
				))}

				{errors.extraPaths?.message && (
					<Text variant="danger" size="xs">
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
						<Plus className="mr-2 h-4 w-4" />
						{t(getKey('extraPaths.addFolder'))}
					</Button>
				</div>
			</div>

			<TextArea
				className="flex"
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
