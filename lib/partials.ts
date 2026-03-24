/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import Debug from 'debug';
import * as fs from 'fs';
import path from 'path';
import handlebars from 'handlebars';
import promisedHandlebars from 'promised-handlebars';
import map from 'lodash/map';
import merge from 'lodash/merge';
import range from 'lodash/range';
import sortBy from 'lodash/sortBy';
import split from 'lodash/split';

import Contract from './contract';
import type { ContractObject } from './types';
import { cartesianProductWith, stripExtraBlankLines } from './utils';

const hb = promisedHandlebars(handlebars);

const debug = Debug('partials');

/**
 * @summary Delimiter to use between contract references
 * @type {String}
 * @private
 */
const REFERENCE_DELIMITER = '+';

/**
 * @summary Calculate the paths to search for a partial given a contract
 * @function
 * @private
 *
 * @param {String} name - partial name (without extension)
 * @param {Object} context - context contract
 * @param {Object} options - options
 * @param {String} options.baseDirectory - partials directory
 * @param {String[]} options.structure - the type hierarchy of the partials directory
 *
 * @returns {String[]} possible partial paths
 *
 * @example
 * const context = new Contract({ ... })
 * context.addChildren([ ... ])
 *
 * const paths = partials.findPartial('my-partial', context, {
 *   baseDirectory: 'my/partials',
 *   // The base directory hierarchy is <distro>/<stack>
 *   structure: [ 'sw.os', 'sw.stack' ]
 * })
 *
 * paths.forEach((path) => {
 *   console.log(`Trying to load ${path}...`)
 * })
 */
export const findPartial = (
	name: string,
	context: Contract,
	options: { baseDirectory: string; structure: string[] },
): string[] => {
	const structure = options.structure;
	const structureReferences = map(structure, (type) => {
		const children = context.getChildrenByType(type);
		const contracts = sortBy(
			children.map((contract) => {
				// We need to replace the alias slug with canonical slug when finding partial
				// since the aliases will use canonical slug to avoid duplication.
				const rawContract = contract.toJSON();
				rawContract.slug = contract.getCanonicalSlug();
				return new Contract(rawContract);
			}),
			(contract) => contract.getSlug(),
		);

		return [
			contracts.map((c) => c.getReferenceString()).join(REFERENCE_DELIMITER),
			contracts.map((c) => c.getSlug()).join(REFERENCE_DELIMITER),
		];
	});
	const products = [
		...cartesianProductWith<string, string[]>(
			structureReferences,
			(accumulator: string[], element: string) => {
				accumulator.push(element);
				return accumulator;
			},
			[[]],
		),
	];

	const slices = range(options.structure.length, 1, -1).reduce<string[][]>(
		(accumulator, slice) =>
			accumulator.concat(products.map((product) => product.slice(0, slice))),
		[],
	);

	const fallbackPaths = structureReferences.reduce<string[][]>(
		(accumulator, _, index, collection) =>
			[map(collection, (x) => x[0]), map(collection, (x) => x[x.length - 1])]
				.map((list) => list.slice(0, index + 1))
				.concat(accumulator),
		[],
	);
	return Array.from(
		new Set(
			(products as Array<Array<string | undefined>>)
				.concat(slices)
				.concat(fallbackPaths)
				.map((references) => [references.join(REFERENCE_DELIMITER), name])
				.concat([[name]])
				.map((paths) => {
					const absolutePath = [options.baseDirectory].concat(paths);
					return `${path.join(...absolutePath)}.tpl`;
				}),
		),
	);
};

hb.registerHelper('import', async (options) => {
	const settings = options.data.root.settings;

	const partialPaths = findPartial(options.hash.partial, settings.context, {
		baseDirectory: path.join(settings.directory, options.hash.combination),
		structure: split(options.hash.combination, REFERENCE_DELIMITER).map((s) =>
			s.trim(),
		),
	});

	for (const partialPath of partialPaths) {
		try {
			const partialContent = await fs.promises.readFile(partialPath, {
				encoding: 'utf8',
			});

			debug(`Using ${partialPath}`);

			// We need to prevent handlebars from encoding the string as HTML,
			// and then we need to parse and recurse through the imported partials,
			// in case they have any interpolation that needs to be resolved.
			const safeContent = new hb.SafeString(
				partialContent.slice(0, partialContent.length - 1),
			);
			const builtContent = await hb.compile(safeContent.toString())(
				options.data.root,
			);
			return new hb.SafeString(builtContent);
		} catch (err) {
			if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
				debug(`Ignoring ${partialPath}`);
				continue;
			}

			throw err;
		}
	}

	throw new Error(`Partial not found: ${options.hash.partial}`);
});

/**
 * @example
 *
 * {{#if (eq hw.device-type.data.media.installation "sdcard")}}
 *   SD card
 * {{/if}}
 * {{#if (eq hw.device-type.data.media.installation "usbkey")}}
 *  USB drive
 * {{/if}}
 *
 * {{#if (and hw.device-type.connectivity.wifi  hw.device-type.connectivity.ethernet)}}
 *   Choose network interface
 * {{/if}}
 */
hb.registerHelper({
	eq: (v1, v2) => v1 === v2,
	ne: (v1, v2) => v1 !== v2,
	lt: (v1, v2) => v1 < v2,
	gt: (v1, v2) => v1 > v2,
	lte: (v1, v2) => v1 <= v2,
	gte: (v1, v2) => v1 >= v2,
	and: (...rest) => rest.every(Boolean),
	or: (...rest) => rest.slice(0, -1).some(Boolean),
});

/**
 * @summary Build a template using a context contract
 * @function
 * @public
 * @memberof module:contrato
 * @name module:contrato.buildTemplate
 *
 * @param {String} template - template
 * @param {Object} context - context contract
 * @param {Object} options - options
 * @param {String} options.directory - partials directory
 * @returns {String} built template
 *
 * @example
 * const template = '....'
 * const context = new Contract({ ... })
 * context.addChildren([ ... ])
 *
 * const result = contrato.buildTemplate(template, context, {
 *   directory: './partials'
 * })
 *
 * console.log(result)
 */
export const buildTemplate = async (
	template: string,
	context: ContractObject,
	options: { directory: string },
): Promise<string> => {
	const data = merge(
		{
			settings: {
				directory: options.directory,
				context,
			},
		},
		context.toJSON().children,
	);

	return stripExtraBlankLines(await hb.compile(template)(data));
};
