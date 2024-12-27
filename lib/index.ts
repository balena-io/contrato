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

export function query(
	universe: Contract,
	layout: BlueprintLayout,
	skeleton: object,
	asIterable: true,
): IterableIterator<Contract>;
export function query(
	universe: Contract,
	layout: BlueprintLayout,
	skeleton: object,
	asIterable?: false,
): Contract[];
export function query(
	universe: Contract,
	layout: BlueprintLayout,
	skeleton: object,
	asIterable = false,
): IterableIterator<Contract> | Contract[] {
	return new Blueprint(layout, skeleton).reproduce(universe, asIterable);
}

export const sequence = (
	universe: Contract,
	layout: BlueprintLayout,
	skeleton: object,
) => new Blueprint(layout, skeleton).sequence(universe);
