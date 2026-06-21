import { zodResolver } from '@hookform/resolvers/zod'
import { useSDK, useSuspenseGraphQL } from '@stump/client'
import { Button, Form } from '@stump/components'
import { graphql, UserPermission } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useCallback, useMemo, useState } from 'react'
import { useForm, useWatch } from 'react-hook-form'

import DirectoryPickerModal from '@/components/DirectoryPickerModal'
import {
	buildSchema,
	CreateOrUpdateLibrarySchema,
	formDefaults,
	intoThumbnailConfig,
	normalizePath,
} from '@/components/library/createOrUpdate'
import { BasicLibraryInformation } from '@/components/library/createOrUpdate/sections'
import { useAppContext } from '@/context'

import { useLibraryManagement } from '../context'

const query = graphql(`
	query BasicSettingsSceneExistingLibraries {
		libraries(pagination: { none: { unpaginated: true } }) {
			nodes {
				id
				name
				path
			}
		}
	}
`)

export default function BasicSettingsScene() {
	const { t } = useLocaleContext()
	const { library, patch } = useLibraryManagement()
	const { sdk } = useSDK()
	const {
		data: {
			libraries: { nodes: libraries },
		},
	} = useSuspenseGraphQL(query, [sdk.cacheKeys.libraryCreateLibraryQuery])
	const { checkPermission } = useAppContext()

	const schema = useMemo(() => buildSchema(libraries, library), [libraries, library])
	const form = useForm<CreateOrUpdateLibrarySchema>({
		defaultValues: formDefaults(library),
		reValidateMode: 'onChange',
		resolver: zodResolver(schema),
	})

	const [showDirectoryPicker, setShowDirectoryPicker] = useState(false)
	const [path, name, description, tags, extraPaths] = useWatch({
		control: form.control,
		name: ['path', 'name', 'description', 'tags', 'extraPaths'],
	})

	const hasChanges = useMemo(() => {
		const currentTagSet = new Set(tags?.map(({ label }) => label) || [])
		const libraryTagSet = new Set(library?.tags?.map(({ name }) => name) || [])
		const currentExtraPaths = (extraPaths ?? []).filter(Boolean).map(normalizePath)
		const libraryExtraPaths = library?.extraPaths ?? []

		return (
			library?.path !== normalizePath(path) ||
			library?.name !== name ||
			library?.description !== description ||
			currentExtraPaths.length !== libraryExtraPaths.length ||
			currentExtraPaths.some((p) => !libraryExtraPaths.includes(p)) ||
			[...currentTagSet].some((tag) => !libraryTagSet.has(tag)) ||
			[...libraryTagSet].some((tag) => !currentTagSet.has(tag))
		)
	}, [library, path, name, description, tags, extraPaths])

	const handleSubmit = useCallback(
		(values: CreateOrUpdateLibrarySchema) => {
			const newExtraPaths = values.extraPaths.filter(Boolean)
			const oldExtraPaths = library.extraPaths ?? []
			const extraPathsChanged =
				newExtraPaths.length !== oldExtraPaths.length ||
				newExtraPaths.some((p) => !oldExtraPaths.includes(p))

			patch({
				config: { thumbnailConfig: intoThumbnailConfig(values.thumbnailConfig) },
				description: values.description,
				extraPaths: newExtraPaths,
				name: values.name,
				path: values.path,
				scanAfterPersist: library.path !== values.path || extraPathsChanged,
				tags: values.tags?.map(({ label }) => label),
			})
		},
		[patch, library],
	)

	return (
		<Form form={form} onSubmit={handleSubmit} fieldsetClassName="flex flex-col gap-12">
			{checkPermission(UserPermission.FileExplorer) && (
				<DirectoryPickerModal
					isOpen={showDirectoryPicker}
					onClose={() => setShowDirectoryPicker(false)}
					startingPath={path}
					onPathChange={(path) => {
						if (path) {
							form.setValue('path', path)
						}
					}}
				/>
			)}

			<BasicLibraryInformation onSetShowDirectoryPicker={setShowDirectoryPicker} />

			<div>
				<Button type="submit" disabled={!hasChanges}>
					{t('librarySettingsScene.basics.form.submit')}
				</Button>
			</div>
		</Form>
	)
}
