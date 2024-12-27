/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

/**
 * Contract variants are syntax sugar that allows the client
 * to express multiple different contracts that share many
 * properties in common as a single object, to avoid repetition.
 */

/**
 * @module variants
 */

import concat from 'lodash/concat';
import isArray from 'lodash/isArray';
import map from 'lodash/map';
import mergeWith from 'lodash/mergeWith';
import omit from 'lodash/omit';
import reduce from 'lodash/reduce';

import type { ContractObject } from './types';

/**
 * @summary The name of the contract property that contains variants
 * @type {String}
 * @constant
 */
const VARIANTS_PROPERTY = 'variants';

/**
 * @summary Build contract variants
 * @function
 * @public
 * @memberof module:variants
 *
 * @description
 * A contract might declare a set of variants of itself as a way
 * to express many different related contracts that share a large
 * set of properties in a single contract to avoid duplication.
 *
 * Before this type of contract can be used, you need to build
 * the variants, and get a list of the final contracts.
 *
 * @param {Object} contract - contract
 * @returns {Object[]} contracts
 *
 * @example
 * const contracts = variants.build({
 *   type: 'distro',
 *   name: 'Debian',
 *   slug: 'debian',
 *   variants: [
 *     {
 *       version: 'wheezy'
 *     },
 *     {
 *       version: 'jessie'
 *     }
 *   ]
 * })
 */
export const build = (contract: ContractObject): ContractObject[] => {
	const variants: ContractObject[] = contract[VARIANTS_PROPERTY] || [];
	const base = omit(contract, [VARIANTS_PROPERTY]) as ContractObject;

	return variants.length === 0
		? [base]
		: reduce(
				variants,
				(accumulator, variation) =>
					concat(
						accumulator,
						map(build(variation), (template) =>
							mergeWith({}, base, template, (object, source) =>
								isArray(object) ? concat(object, source) : undefined,
							),
						),
					),
				[],
			);
};
