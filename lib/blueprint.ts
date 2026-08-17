/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import reduce from 'lodash/reduce';

import Contract from './contract';
import type { Cardinality } from './cardinality';
import { parse } from './cardinality';
import type { BlueprintLayout, BlueprintObject, ContractObject } from './types';
import { BLUEPRINT, CONTEXT } from './types';
import {
	cartesianProductWith,
	flatten as flattenIterator,
	filter as filterIterator,
} from './utils';

/** A single parsed blueprint layout selector. */
interface BlueprintSelector {
	cardinality: Cardinality;
	filter?: any;
	type: string;
	version?: string;
}

/** A finite/infinite grouping of parsed selectors keyed by contract type. */
interface BlueprintLayoutGroup {
	selectors: Record<string, BlueprintSelector[]>;
	types: Set<string>;
}

/** The parsed blueprint layout. */
interface ParsedBlueprintLayout {
	types: Set<string>;
	finite: BlueprintLayoutGroup;
	infinite: BlueprintLayoutGroup;
}

export default class Blueprint extends Contract {
	declare protected $raw: BlueprintObject;

	/** The parsed blueprint layout. */
	private readonly $layout: ParsedBlueprintLayout;

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
	 * @description Creates a new Blueprint with a given layout and skeleton
	 *
	 * If no skeleton is given, a default `{"type": "meta.context"}` skeleton will be used.
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
	constructor(layout: BlueprintLayout, skeleton?: ContractObject) {
		super({
			type: BLUEPRINT,
			skeleton: skeleton ?? { type: CONTEXT },
		});

		const initialLayout: ParsedBlueprintLayout = {
			types: new Set<string>(),
			finite: {
				selectors: {},
				types: new Set<string>(),
			},
			infinite: {
				selectors: {},
				types: new Set<string>(),
			},
		};

		this.$layout = reduce(
			layout,
			(accumulator, value, type) => {
				const selector: BlueprintSelector = {
					cardinality: parse(value.cardinality ?? value),
					// Array has its own `filter` function, which we need to ignore
					filter: Array.isArray(value) ? undefined : value.filter,
					type: value.type ?? type,
					version: value.version,
				};

				const group = selector.cardinality.finite ? 'finite' : 'infinite';
				accumulator[group].selectors[selector.type] = [
					...(accumulator[group].selectors[selector.type] ?? []),
					selector,
				];
				accumulator[group].types.add(selector.type);
				accumulator.types.add(selector.type);

				return accumulator;
			},
			initialLayout,
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
	 * @param {Contract} contract - contract
	 * @returns {Iterable<Contract>} - an iterable over the valid contexts
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
		const layout = this.$layout;
		const skeleton = this.$raw.skeleton;

		const combinations = reduce(
			layout.finite.selectors,
			(accumulator: Contract[][][], value) => {
				let internalAccumulator = accumulator;
				for (const option of value) {
					internalAccumulator = internalAccumulator.concat([
						contract.getChildrenCombinations(option),
					]);
				}
				return internalAccumulator;
			},
			[],
		);

		const productIterator = cartesianProductWith<
			Contract[],
			Contract | Contract[]
		>(
			combinations,
			(accumulator, element) => {
				if (accumulator instanceof Contract) {
					const prodContext = new Contract(skeleton);

					prodContext.addChildren(element.concat(accumulator.getChildren()));

					if (
						!prodContext.areChildrenSatisfied({
							types: prodContext.getChildrenTypes(),
						})
					) {
						return undefined;
					}

					return prodContext;
				}

				const context = new Contract(skeleton);

				return context.addChildren(accumulator.concat(element));
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

			context.addChildren(contracts);

			for (const reference of contracts) {
				if (
					!context.satisfiesChildContract(reference, {
						types: layout.types,
					})
				) {
					context.removeChild(reference);
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
