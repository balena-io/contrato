/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import reduce from 'lodash/reduce';
import set from 'lodash/set';

import type { ContractObject, ChildrenTree } from './types';
import type Contract from './contract';
import { setFirst } from './utils';

/**
 * @module children-tree
 */

/**
 * @summary Build a plain children tree out of a built contract
 * @function
 * @public
 * @memberof module:children-tree
 *
 * @param {Object} contract - contract
 * @returns {Object} children tree
 *
 * @example
 * const contract = new Contract({ ... })
 * contract.addChildren([ ... ])
 *
 * const tree = childrenTree.build(contract)
 */
export const build = (contract: Contract): ChildrenTree => {
	const tree = {};

	for (const type of contract.metadata.children.types) {
		if (contract.metadata.children.byType[type].size === 1) {
			const hash = setFirst<string>(contract.metadata.children.byType[type]);
			if (hash === undefined) {
				throw new Error('Error retrieving child');
			}

			const child = contract.getChildByHash(hash);
			if (child === undefined) {
				throw new Error('Error retrieving child');
			}
			set(tree, type, child.toJSON());
			continue;
		}

		for (const slug of Object.keys(
			contract.metadata.children.byTypeSlug[type],
		)) {
			const sources: object[] = [];
			for (const hash of contract.metadata.children.byTypeSlug[type][slug]) {
				const child = contract.getChildByHash(hash);
				if (child === undefined) {
					throw new Error('Error retrieving child');
				}
				sources.push(child.toJSON());
			}

			if (sources.length === 0) {
				continue;
			}

			set(tree, `${type}.${slug}`, sources.length === 1 ? sources[0] : sources);
		}
	}

	return tree;
};

/**
 * @summary Get all source contract children from a children tree
 * @function
 * @public
 * @memberof module:children-tree
 *
 * @param {Object} tree - children tree
 * @returns {Object[]} children source contracts
 *
 * @example
 * const tree = {
 *   hw: {
 *     'device-type': { ... }
 *   },
 *   sw: {
 *     distro: {
 *       debian: { ... },
 *       fedora: { ... }
 *     }
 *   }
 * }
 *
 * childrenTree.getAll(tree).forEach((sourceContract) => {
 *   console.log(sourceContract.slug, sourceContract.version)
 * })
 */
export const getAll = (tree: any): ContractObject[] =>
	reduce(
		tree,
		(accumulator, value) => {
			if (!value.slug) {
				const out = accumulator.concat(getAll(value));
				return out;
			}

			accumulator.push(value);
			return accumulator;
		},
		[] as ContractObject[],
	);
