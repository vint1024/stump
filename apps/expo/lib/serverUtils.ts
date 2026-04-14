import { SavedServer } from '~/stores/savedServer'

// todo: assets for each (just a favicon)

// todo: more common ones?
type KnownServer = 'stump' | 'codex' | 'kavita' | 'komga'

// todo: figure out how to ident
// ^ my thought is using for logos for servers screen, for stump idk yet. user avatar or stump logo

// ^^ alternatively, i could just _not_ store the meta of what type of server and just _assume_ /favicon.ico will return
// something

export async function identifyKnownServer(server: SavedServer): Promise<KnownServer | null> {
	if (server.kind === 'stump') {
		return 'stump'
	}

	const knownServer = (
		await Promise.all([
			checkForCodex(server.url),
			checkForKavita(server.url),
			checkForKomga(server.url),
		])
	).find((result): result is Exclude<typeof result, false> => result !== false)

	return knownServer ?? null
}

async function checkForCodex(url: string): Promise<'codex' | false> {
	return false
}

async function checkForKavita(url: string): Promise<'kavita' | false> {
	return false
}

async function checkForKomga(url: string): Promise<'komga' | false> {
	return false
}
