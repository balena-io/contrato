/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

/**
 * @module contrato
 * @public
 */

import { BlueprintLayout, BlueprintObject, ContractObject } from './types';
import Contract from './contract';
import Blueprint from './blueprint';
import Universe from './universe';
import { buildTemplate } from './partials';
import { parse as parseCardinality } from './cardinality';

export {
	BlueprintLayout,
	ContractObject,
	BlueprintObject,
	Contract,
	Blueprint,
	Universe,
	buildTemplate,
	parseCardinality,
};

/**
 * @summary Reproduce the query in a universe and return as an iterable
 * @function
 * @public
 * @memberof module:contrato
 * @name module:contrato.query
 * @description
 * This method will generate a set of contexts that consist of
 * every possible valid combination that matches the blueprint
 * layout. It uses depth first search to calculate the product of
 * contract combinations and returns the results as an iterable.
 * This allows to reduce the memory usage when dealing with a large
 * universe of contracts.
 *
 * @param universe - composite contract
 * @param layout - an object describing the query using a set of cardinality operators
 * @param skeleton - contract skeleton for the returned contexts
 * @returns an iterable over the valid contexts
 *
 ** @example
 * import {query, Contract} from 'contrato';
 *
 * const contract = new Contract({ ... })
 * contract.addChildren([ ... ])
 *
 * const contexts = query(contract, {
 *			'hw.device-type': 1,
 *			'arch.sw': {
 *				cardinality: 1,
 *				filter: { type: 'object', properties: { slug: { const: 'armv7hf' } } },
 *			},
 *		});
 *
 * for (const context of contexts) {
 *		console.log(context.toJSON());
 * }
 */
export function query(
	universe: Contract,
	layout: BlueprintLayout,
	skeleton: object,
): IterableIterator<Contract> {
	return new Blueprint(layout, skeleton).reproduce(universe);
}

export const sequence = (
	universe: Contract,
	layout: BlueprintLayout,
	skeleton: object,
) => new Blueprint(layout, skeleton).sequence(universe);
