import path from 'path';
import { promises as fs } from 'fs';
import type { Stats } from 'fs';

import Contract from './contract';
import { UNIVERSE } from './types';

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
	filter: (filePath: string, stat: Stats) => boolean;
	canonicalOnly: boolean;
}

export class Universe extends Contract {
	constructor() {
		super({ type: UNIVERSE });
	}

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
