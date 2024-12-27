import path from 'path';
import { promises as fs } from 'fs';
import type { Stats } from 'fs';

import Contract from './contract';
import { UNIVERSE } from './types';

/**
 * @summary recursively find all files under the directory that match the given filter
 * @function
 * @memberof module:universe
 *
 * @param dir - base directory to start the search
 * @param filter - filtering function to indicate that a file should be selected
 */
async function findFiles(
	dir: string,
	filter: (filePath: string, stat: Stats) => boolean = () => true,
): Promise<string[]> {
	const allFiles = await fs.readdir(dir, { recursive: true });
	const filePaths: string[] = [];
	for (const fileName of allFiles) {
		const filePath = path.join(dir, fileName);
		const stat = await fs.stat(filePath);
		if (!stat.isDirectory() && filter(filePath, stat)) {
			filePaths.push(filePath);
		}
	}

	return filePaths;
}

interface FromFsOptions {
	/**
	 * Additional filters to apply to json files when loading a universe from FS
	 */
	filter: (filePath: string, stat: Stats) => boolean;

	/**
	 * Only load the canonical version of the contract and ignore
	 * aliases
	 */
	canonicalOnly: boolean;
}

export class Universe extends Contract {
	constructor() {
		super({ type: UNIVERSE });
	}

	/**
	 * @summary recursively looks up all json files under a directory and adds them
	 * to the universe
	 * @function
	 * @static
	 * @name module:contrato.Universe.fromFs
	 * @public
	 *
	 * @param dir full path of the directory to load
	 * @param options additional configuration for the search and build process
	 *
	 */
	static async fromFs(
		dir: string,
		{ filter = () => true, canonicalOnly = false }: Partial<FromFsOptions> = {},
	): Promise<Universe> {
		const allFiles = await findFiles(
			dir,
			(filePath, stat) =>
				path.extname(filePath) === '.json' && filter(filePath, stat),
		);

		const { default: pMap } = await import('p-map');

		const universe = new Universe();
		const children = (
			await pMap(
				allFiles,
				async (file) => {
					const contents = await fs.readFile(file, { encoding: 'utf8' });
					let source = JSON.parse(contents);
					if (canonicalOnly) {
						// Ignore aliases
						const { aliases, ...obj } = source;
						source = obj;
					}
					return Contract.build(source);
				},
				{ concurrency: 10 },
			)
		).flat();

		universe.addChildren(children);

		return universe;
	}
}

export default Universe;
