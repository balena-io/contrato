import { BlueprintLayout, BlueprintObject, ContractObject } from './types';
import Contract from './contract';
import type { ContractMatcher } from './contract';
import Blueprint from './blueprint';
import Universe from './universe';
import { buildTemplate } from './partials';

export {
	BlueprintLayout,
	ContractObject,
	BlueprintObject,
	ContractMatcher,
	Contract,
	Blueprint,
	Universe,
	buildTemplate,
};

export function query(
	universe: Contract,
	layout: BlueprintLayout,
	skeleton: object,
): IterableIterator<Contract> {
	return new Blueprint(layout, skeleton).reproduce(universe);
}
