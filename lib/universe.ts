import path from 'path';
import { promises as fs } from 'fs';
import type { Stats } from 'fs';

import Contract from './contract';
import { UNIVERSE } from './types/types';

/**
 * @summary Run the callback function concurrently on elements from the given iterator
 * @function
 * @memberof module:universe
 *
 * Gets up to `concurrency` elements from the given iterator and apply the asynchronous function
 * concurrently using `Promise.all`.
 *
 * If at any point the call to the callback fails, the function will throw the error
 *
 * @param it - iterator of elements to go traverse
 * @param callbackFn - a function to execute for each element in the iterator
 * @param concurrency - number of elements to apply the function at the same time. Default to 1
 */
export async function concurrentForEach<T>(
	it: IterableIterator<T>,
	callbackFn: (t: T) => PromiseLike<void>,
	concurrency = 1,
): Promise<void> {
	const run = async () => {
		const next = it.next();
		if (next.value && !next.done) {
			await callbackFn(next.value);
			await run();
		}
	};
	const runs = [];
	for (let i = 0; i < concurrency; i++) {
		runs.push(run());
	}
	await Promise.all(runs);
}

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

		const universe = new Universe();
		const children: Contract[] = [];
		await concurrentForEach(
			allFiles.values(),
			async (file) => {
				const contents = await fs.readFile(file, { encoding: 'utf8' });
				let source = JSON.parse(contents);

				if (canonicalOnly) {
					// Ignore aliases
					const { aliases, ...obj } = source;
					source = obj;
				}

				children.push(...Contract.build(source));
			},
			10,
		);

		universe.addChildren(children);

		return universe;
	}
}

export default Universe;
