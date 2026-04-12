import reduce from 'lodash/reduce';

import Contract from './contract';
import { parse } from './cardinality';
import type { BlueprintLayout } from './types';
import {
	cartesianProductWith,
	flatten as flattenIterator,
	filter as filterIterator,
} from './utils';

interface LayoutMetadata {
	types: Set<string>;
	finite: {
		selectors: { [type: string]: any[] };
		types: Set<string>;
	};
	infinite: {
		selectors: { [type: string]: any[] };
		types: Set<string>;
	};
}

export default class Blueprint {
	readonly skeleton: any;
	readonly layout: LayoutMetadata;

	constructor(rawLayout: BlueprintLayout, skeleton?: any) {
		this.skeleton = skeleton;
		this.layout = reduce(
			rawLayout,
			(accumulator, value, type) => {
				const selector = {
					cardinality: parse(value.cardinality ?? value) as ReturnType<
						typeof parse
					> & { type: string },
					filter: Array.isArray(value) ? undefined : value.filter,
					type: value.type ?? type,
					version: value.version,
				};

				selector.cardinality.type = selector.type;

				const group = selector.cardinality.finite ? 'finite' : 'infinite';
				accumulator[group].selectors[selector.type] = [
					...(accumulator[group].selectors[selector.type] || []),
					selector,
				];
				accumulator[group].types.add(selector.type);
				accumulator.types.add(selector.type);

				return accumulator;
			},
			{
				types: new Set(),
				finite: {
					selectors: {} as { [type: string]: any[] },
					types: new Set(),
				},
				infinite: {
					selectors: {} as { [type: string]: any[] },
					types: new Set(),
				},
			} as LayoutMetadata,
		);
	}

	reproduce(contract: Contract): IterableIterator<Contract> {
		const layout = this.layout;
		const combinations = reduce(
			layout.finite.selectors,
			(accumulator, value) => {
				let internalAccumulator = accumulator;
				for (const option of value) {
					internalAccumulator = internalAccumulator.concat([
						contract.getChildrenCombinations(option),
					]);
				}
				return internalAccumulator;
			},
			[] as Contract[][][],
		);

		const skeleton = this.skeleton;

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
							types: layout.finite.types,
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
