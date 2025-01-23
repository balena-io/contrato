/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import concat from 'lodash/concat';
import forEach from 'lodash/forEach';
import reduce from 'lodash/reduce';

import Contract from './contract';
import { parse } from './cardinality';
import type { BlueprintLayout, BlueprintObject } from './types';
import { BLUEPRINT } from './types';
import {
	cartesianProductWith,
	flatten as flattenIterator,
	filter as filterIterator,
} from './utils';

export default class Blueprint extends Contract {
	/**
	 * @summary A blueprint contract data structure
	 * @name Blueprint
	 * @memberof module:contrato
	 * @class
	 * @public
	 *
	 * @param {Object} layout - the blueprint layout
	 * @param {Object} skeleton - the blueprint skeleton
	 *
	 * @example
	 * const blueprint = new Blueprint({
	 *   'arch.sw': 1,
	 *   'hw.device-type': 1
	 * }, {
	 *   type: 'my-context',
	 *   slug: '{{children.arch.sw.slug}}-{{children.hw.device-type.slug}}'
	 * })
	 */
	constructor(layout: BlueprintLayout, skeleton?: any) {
		super({
			type: BLUEPRINT,
			skeleton,
			layout,
		} as BlueprintObject);

		this.metadata.layout = reduce(
			this.raw.layout,
			(accumulator: any, value, type) => {
				const selector = {
					cardinality: parse(value.cardinality || value) as any,
					// Array has its own `filter` function, which we need to ignore
					filter: Array.isArray(value) ? undefined : value.filter,
					type: value.type || type,
					version: value.version,
				};

				selector.cardinality.type = selector.type;

				const group = selector.cardinality.finite ? 'finite' : 'infinite';
				accumulator[group].selectors[selector.type] = concat(
					accumulator[group].selectors[selector.type] || [],
					[selector],
				);
				accumulator[group].types.add(selector.type);
				accumulator.types.add(selector.type);

				return accumulator;
			},
			{
				types: new Set(),
				finite: {
					selectors: {},
					types: new Set(),
				},
				infinite: {
					selectors: {},
					types: new Set(),
				},
			},
		);
	}

	/**
	 * @summary Reproduce the blueprint in a universe and return as an iterable
	 * @function
	 * @name module:contrato.Blueprint#reproduce
	 * @public
	 *
	 * @description
	 * This method will generate a set of contexts that consist of
	 * every possible valid combination that matches the blueprint
	 * layout. It uses depth first search to calculate the product of
	 * contract combinations and returns the results as an iterable.
	 * This allows to reduce the memory usage when dealing with a large
	 * universe of contracts.
	 *
	 * @param {Object} contract - contract
	 * @returns {Iterable<Object>} - an iterable over the valid contexts
	 *
	 * @example
	 * const contract = new Contract({ ... })
	 * contract.addChildren([ ... ])
	 *
	 * const blueprint = new Blueprint({
	 *   'hw.device-type': 1,
	 *   'arch.sw': 1
	 * })
	 *
	 * const contexts = blueprint.reproduce(contract)
	 * for (const context of contexts) {
	 *   console.log(context.toJSON());
	 * }
	 */
	reproduce(contract: Contract): IterableIterator<Contract> {
		const layout = this.metadata.layout;
		const combinations = reduce(
			layout.finite.selectors,
			(accumulator, value) => {
				let internalAccumulator = accumulator;
				forEach(value, (option) => {
					internalAccumulator = internalAccumulator.concat([
						contract.getChildrenCombinations(option),
					]);
				});
				return internalAccumulator;
			},
			[] as Contract[][][],
		);

		const productIterator = cartesianProductWith<
			Contract[],
			Contract | Contract[]
		>(
			combinations,
			(accumulator, element) => {
				if (accumulator instanceof Contract) {
					const prodContext = new Contract(this.raw.skeleton, {
						hash: false,
					});

					prodContext.addChildren(element.concat(accumulator.getChildren()), {
						rehash: false,
					});

					// TODO: Make sure this is cached
					if (
						!prodContext.areChildrenSatisfied({
							types: prodContext.getChildrenTypes(),
						})
					) {
						return undefined;
					}

					return prodContext;
				}

				// If the accumulator is an array of contracts
				const context = new Contract(this.raw.skeleton, {
					hash: false,
				});

				return context.addChildren(accumulator.concat(element), {
					rehash: false,
				});
			},
			[[]],
		);

		return filterIterator(flattenIterator(productIterator), (context: any) => {
			const references = context.getChildrenCrossReferencedContracts({
				from: contract,
				types: layout.infinite.types,
			});

			const contracts =
				references.length === 0
					? contract.getChildren({
							types: layout.infinite.types,
						})
					: references;

			context.addChildren(contracts, {
				rehash: false,
			});

			for (const reference of contracts) {
				if (
					!context.satisfiesChildContract(reference, {
						types: layout.types,
					})
				) {
					context.removeChild(reference, {
						rehash: false,
					});
				}
			}

			if (
				!context.areChildrenSatisfied({
					types: layout.infinite.types,
				})
			) {
				return false;
			}

			context.interpolate();
			return true;
		});
	}
}
