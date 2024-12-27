/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import clone from 'lodash/clone';
import concat from 'lodash/concat';
import fill from 'lodash/fill';
import filter from 'lodash/filter';
import flatMap from 'lodash/flatMap';
import flatten from 'lodash/flatten';
import forEach from 'lodash/forEach';
import includes from 'lodash/includes';
import isEmpty from 'lodash/isEmpty';
import isEqual from 'lodash/isEqual';
import reduce from 'lodash/reduce';
import uniqWith from 'lodash/uniqWith';
import { compare } from 'semver';

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
					filter: value.filter,
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
	 * @summary Reproduce the blueprint in a universe
	 * @function
	 * @name module:contrato.Blueprint#reproduce
	 * @public
	 *
	 * @description
	 * This method will generate a set of contexts that consist of
	 * every possible valid combination that matches the blueprint
	 * layout.
	 *
	 * @param {Object} contract - contract
	 * @returns {Object[]} valid contexts
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
	 *
	 * contexts.forEach((context) => {
	 *   console.log(context.toJSON())
	 * })
	 */
	sequence(
		contract: Contract,
		options = {
			allowRequirements: true,
		},
	): Contract[] {
		const layout = this.metadata.layout;

		const combinations = reduce(
			layout.finite.selectors,
			(accumulator, value) => {
				let internalAccumulator = accumulator;
				forEach(value, (option) => {
					const combi = uniqWith(
						contract.getChildrenCombinations(option),
						(left: Contract[], right: Contract[]) => {
							return isEqual(left[0].raw, right[0].raw);
						},
					);
					internalAccumulator = internalAccumulator.concat([combi]);
				});
				return internalAccumulator;
			},
			[] as Contract[][][],
		);

		forEach(combinations, (dimension) => {
			dimension.sort((left, right) => {
				return compare(left[0].raw.version, right[0].raw.version);
			});
		});

		const currentPointer = new Array<number>(combinations.length);
		fill(currentPointer, 0);

		const bestPointer = new Array<number>(combinations.length);
		for (let idx = 0; idx < combinations.length; idx++) {
			bestPointer[idx] = combinations[idx].length - 1;
		}

		const buildContextFromPointer = (pointer: number[]) => {
			const context = new Contract(this.raw.skeleton, {
				hash: false,
			});
			const combination = [] as Contract[][];
			for (let idx = 0; idx < combinations.length; idx++) {
				combination.push(combinations[idx][pointer[idx]]);
			}
			context.addChildren(flatten(combination), {
				rehash: true,
			});

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

			context.interpolate();

			const requirements = context.getAllNotSatisfiedChildRequirements();
			const newRequirements = uniqWith(
				filter(concat(context.raw.requires, requirements)),
				isEqual,
			);
			if (newRequirements && !isEmpty(newRequirements)) {
				if (!options.allowRequirements) {
					return null;
				}
				context.raw.requires = newRequirements;
				context.interpolate();
			}

			const childCapabilities = filter(
				uniqWith(
					flatMap(context.getChildren(), (v: any) => v.raw.capabilities),
					isEqual,
				),
			);

			if (childCapabilities && !isEmpty(childCapabilities)) {
				context.raw.capabilities = childCapabilities;
				context.interpolate();
			}

			return context;
		};
		/**
		 * This will validate if a certain set of possibilities is good
		 */
		const checkSolutions = (pointer: number[]) => {
			const context = buildContextFromPointer(pointer);
			return !context
				? false
				: context.areChildrenSatisfied({
						types: layout.types,
					});
		};

		const checked: number[][] = [];

		const pointerValue = (pointer: number[]) =>
			reduce(pointer, (sum, value) => sum + value, 0);

		let currentBestPointer = new Array<number>(combinations.length);
		fill(currentBestPointer, 0);
		const currentBestPointerValue = pointerValue(currentBestPointer);
		let currentBestPath = [] as number[][];

		const isValidPointer = (pointer: number[]) => {
			if (includes(checked, pointer)) {
				return false;
			}
			for (let idx = 0; idx < combinations.length; idx++) {
				if (pointer[idx] > bestPointer[idx]) {
					return false;
				}
			}
			if (!checkSolutions(pointer)) {
				return false;
			}
			return true;
		};
		/**
		 * Implements a simple recursive depth first search on the graph
		 * @param {*} combos asd asd
		 * @param {*} pointer asdas
		 */
		const search = (
			combos: Contract[][][],
			pointer: number[],
			path: number[][],
		) => {
			checked.push(pointer);
			for (let idx = 0; idx < combos.length; idx++) {
				const possiblePointer = clone(pointer);
				possiblePointer[idx] += 1;
				if (isValidPointer(possiblePointer)) {
					const currentPath = clone(path);
					currentPath.push(possiblePointer);
					if (isEqual(possiblePointer, bestPointer)) {
						currentBestPath = currentPath;
						return true;
					}
					const solutionValue = pointerValue(possiblePointer);
					if (solutionValue > currentBestPointerValue) {
						currentBestPointer = possiblePointer;
						currentBestPath = currentPath;
					}
					if (search(combos, possiblePointer, currentPath)) {
						return true;
					}
					return false;
				}
			}
			return false;
		};

		if (isValidPointer(currentPointer)) {
			currentBestPointer = currentPointer;
			currentBestPath = [currentPointer];
			search(combinations, currentPointer, [currentPointer]);
		}

		return reduce(
			currentBestPath,
			(seq, pointer) => {
				const context = buildContextFromPointer(pointer);
				if (context) {
					if (
						!context.areChildrenSatisfied({
							types: layout.infinite.types,
						})
					) {
						return seq;
					}

					context.interpolate();
					seq.push(context);
				}
				return seq;
			},
			[] as Contract[],
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
