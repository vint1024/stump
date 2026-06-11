import { APIBase } from '../base'
import { ClassQueryKeys } from './types'
import { createRouteURLHandler } from './utils'

/**
 * The root route for the epub API
 */
const EPUB_ROUTE = '/epub'
/**
 * A helper function to format the URL for epub API routes with optional query parameters
 */
const epubURL = createRouteURLHandler(EPUB_ROUTE)

/**
 * The epub API controller, used for interacting with the epub endpoints of the Stump API
 */
export class EpubAPI extends APIBase {
	/**
	 * A helper to get the service URL for the epub API scoped to a specific epub ID
	 */
	epubServiceURL(id: string): string {
		return epubURL(`/${id}`)
	}

	/**
	 * The base URL for streaming raw resources out of an epub file. The trailing
	 * slash matters: epub.js consumes this as an unpacked "directory" input and
	 * resolves META-INF/container.xml (and then every internal resource) below it.
	 * Unlike the media download URL, this route does not require the DownloadFile
	 * permission — reading a book is not downloading it
	 */
	resourceBaseURL(id: string): string {
		return `${this.withServiceURL(epubURL(`/${id}/resource`))}/`
	}

	/**
	 * The URL of the Readium Web Publication Manifest for an epub
	 */
	manifestURL(id: string): string {
		return this.withServiceURL(epubURL(`/${id}/manifest.json`))
	}

	/**
	 * Fetch a resource from an epub by its ID and resource ID
	 */
	async fetchResource({
		id,
		root = 'META-INF',
		resourceId,
	}: {
		id: string
		root?: string
		resourceId: string
	}): Promise<string> {
		const { data: resource } = await this.api.axios.get<string>(
			epubURL(`${id}/${root}/${resourceId}`),
		)
		return resource
	}

	/**
	 * The query keys for the epub API, used for query caching on a client (e.g. react-query)
	 */
	get keys(): ClassQueryKeys<InstanceType<typeof EpubAPI>> {
		return {
			epubServiceURL: 'epub.serviceURL',
			fetchResource: 'epub.fetchResource',
			manifestURL: 'epub.manifestURL',
			resourceBaseURL: 'epub.resourceBaseURL',
		}
	}
}
