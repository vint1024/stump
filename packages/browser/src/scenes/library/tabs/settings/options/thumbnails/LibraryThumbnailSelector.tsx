import { useGraphQLMutation, useSDK } from '@stump/client'
import { Button, Dialog, Label, PickSelect, Text } from '@stump/components'
import { graphql, LibraryThumbnailSelectorUpdateMutation } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { Suspense, useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'

import EditThumbnailDropdown from '@/components/thumbnail/EditThumbnailDropdown'
import BookPageGrid from '@/scenes/book/settings/BookPageGrid'
import { useLibraryContext } from '@/scenes/library/context'
import SeriesBookGrid, { SelectedBook } from '@/scenes/series/tabs/settings/SeriesBookGrid'

import LibrarySeriesGrid, { SelectedSeries } from '../../LibrarySeriesGrid'

// TODO: Redesign this ugly shit

const updateMutation = graphql(`
	mutation LibraryThumbnailSelectorUpdate($id: ID!, $input: UpdateThumbnailInput!) {
		updateLibraryThumbnail(id: $id, input: $input) {
			id
			thumbnail {
				url
			}
		}
	}
`)

const uploadMutation = graphql(`
	mutation LibraryThumbnailSelectorUpload($id: ID!, $file: Upload!) {
		uploadLibraryThumbnail(id: $id, file: $file) {
			id
			thumbnail {
				url
			}
		}
	}
`)

type OnSuccessData = PickSelect<LibraryThumbnailSelectorUpdateMutation, 'updateLibraryThumbnail'>

export default function LibraryThumbnailSelector() {
	const { t } = useLocaleContext()
	const { sdk } = useSDK()
	const [selectedSeries, setSelectedSeries] = useState<SelectedSeries>()
	const [selectedBook, setSelectedBook] = useState<SelectedBook>()
	const [page, setPage] = useState<number>()

	const [isOpen, setIsOpen] = useState(false)

	const { library } = useLibraryContext()

	const onSuccess = useCallback(
		({ thumbnail }: OnSuccessData) =>
			sdk.axios.get(thumbnail.url, {
				headers: {
					'Cache-Control': 'no-cache',
					Pragma: 'no-cache',
					Expires: '0',
				},
			}),
		[sdk],
	)

	const { mutateAsync: patchThumbnail, isPending: isPatchingThumbnail } = useGraphQLMutation(
		updateMutation,
		{
			onSuccess: (data) => onSuccess(data.updateLibraryThumbnail),
		},
	)

	const { mutateAsync: uploadThumbnail, isPending: isUploadingThumbnail } = useGraphQLMutation(
		uploadMutation,
		{
			onSuccess: (data) => onSuccess(data.uploadLibraryThumbnail),
		},
	)

	const handleOpenChange = (nowOpen: boolean) => {
		if (!nowOpen) {
			setIsOpen(false)
		}
	}

	const handleCancel = () => {
		if (page) {
			setPage(undefined)
		}
		setIsOpen(false)
	}

	const handleUploadImage = useCallback(
		async (file: File) => {
			try {
				await uploadThumbnail({ id: library.id, file })
				setIsOpen(false)
			} catch (error) {
				console.error(error)
				toast.error(
					t(
						'scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.uploadFailed',
					),
				)
			}
		},
		[library.id, uploadThumbnail, t],
	)

	const handleConfirm = useCallback(async () => {
		if (!selectedBook || !page) return

		try {
			await patchThumbnail({ id: library.id, input: { mediaId: selectedBook.id, page } })
			setIsOpen(false)
		} catch (error) {
			console.error(error)
			toast.error(
				t('scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.updateFailed'),
			)
		}
	}, [patchThumbnail, selectedBook, page, library.id, t])

	useEffect(() => {
		return () => {
			setSelectedSeries(undefined)
			setSelectedBook(undefined)
			setPage(undefined)
		}
	}, [isOpen])

	const renderContent = () => {
		if (selectedBook) {
			return (
				<BookPageGrid
					bookId={selectedBook.id}
					pages={selectedBook.pages}
					selectedPage={page}
					onSelectPage={setPage}
				/>
			)
		} else if (selectedSeries) {
			return <SeriesBookGrid seriesId={selectedSeries.id} onSelectBook={setSelectedBook} />
		} else {
			return <LibrarySeriesGrid libraryId={library.id} onSelectSeries={setSelectedSeries} />
		}
	}

	const renderDescription = () => {
		if (selectedBook) {
			return t(
				'scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.choosePage',
			)
		} else if (selectedSeries) {
			return t(
				'scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.selectBook',
			)
		} else {
			return t(
				'scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.selectSeries',
			)
		}
	}

	const renderGoBack = () => {
		if (!selectedBook && !selectedSeries) return null

		return (
			<span
				className="ml-2 cursor-pointer underline"
				onClick={() => {
					setPage(undefined)
					if (selectedBook) {
						setSelectedBook(undefined)
					} else if (selectedSeries) {
						setSelectedSeries(undefined)
					}
				}}
			>
				{t('scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.goBack')}
			</span>
		)
	}

	return (
		<div className="gap-4 flex flex-col">
			<div>
				<Label>
					{t('scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.label')}
				</Label>
				<Text size="sm" variant="muted">
					{t(
						'scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.labelDescription',
					)}
				</Text>
			</div>

			<div>
				<EditThumbnailDropdown
					onChooseSelector={() => setIsOpen(true)}
					onUploadImage={handleUploadImage}
				/>
			</div>

			<Dialog open={isOpen} onOpenChange={handleOpenChange}>
				<Dialog.Content size="xl">
					<Dialog.Header>
						<Dialog.Title>
							{t(
								'scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.dialogTitle',
							)}
						</Dialog.Title>
						<Dialog.Description>
							{renderDescription()}
							{renderGoBack()}
						</Dialog.Description>
						<Dialog.Close onClick={() => setIsOpen(false)} />
					</Dialog.Header>

					<Suspense>{renderContent()}</Suspense>

					<Dialog.Footer>
						<Button variant="default" onClick={handleCancel}>
							{t('scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.cancel')}
						</Button>
						<Button
							variant="primary"
							onClick={handleConfirm}
							disabled={!selectedSeries || !selectedBook || !page}
							isLoading={isPatchingThumbnail || isUploadingThumbnail}
						>
							{t(
								'scenes.library.tabs.settings.options.thumbnails.LibraryThumbnailSelector.confirm',
							)}
						</Button>
					</Dialog.Footer>
				</Dialog.Content>
			</Dialog>
		</div>
	)
}
