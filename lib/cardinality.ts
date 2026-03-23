/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import initial from 'lodash/initial';
import isEqual from 'lodash/isEqual';
import isInteger from 'lodash/isInteger';

/**
 * @module cardinality
 */

/**
 * @summary The length of a cardinality ordered pair (a tuple)
 * @type {Number}
 * @constant
 */
const ORDERED_LIST_LENGTH = 2;

/**
 * @summary Parse a contracts cardinality tuple/string/number
 * @function
 * @public
 * @memberof module:cardinality
 *
 * @description
 * A cardinality is usually represented with a tuple that defines
 * a range of integers. On top of that, this function supports the
 * following syntax sugar, assuming `x` in an integer:
 *
 * - `x` -> `[ x, x ]`
 * - `*` -> `[ 0, Infinity ]`
 * - `?` -> `[ 0, 1 ]`
 * - `1?` -> `[ 0, 1 ]`
 * - `'x'` -> `[ x, x ]`
 * - `x+` -> `[ x, Infinity ]`
 * - `[ x, '*' ]` -> `[ x, Infinity ]`
 *
 * @param {(Array|String|Number)} input - cardinality
 * @returns {Object} parsed cardinality
 *
 * @example
 * const result = cardinality.parse([ 1, 2 ])
 * console.log(result.from)
 * console.log(result.to)
 *
 * if (result.finite) {
 *   console.log('This is a finite cardinality')
 * }
 */
export const parse = (
	input: Array<string | number> | string | number,
): { from: number; to: number; finite: boolean } => {
	if (typeof input === 'number') {
		return parse([input, input]);
	}

	if (typeof input === 'string') {
		const normalizedInput = input.trim();

		if (normalizedInput === '*') {
			return parse([0, Infinity]);
		}

		if (normalizedInput === '?' || /^1\s*\?$/.test(normalizedInput)) {
			return parse([0, 1]);
		}

		if (/^[0-9]+$/.test(normalizedInput)) {
			const num = parseInt(normalizedInput, 10);

			return parse([num, num]);
		}

		return parse([parseInt(initial(normalizedInput).join(''), 10), Infinity]);
	}

	const [from, to] = input;

	// Alias an asterisk to Infinity
	if (typeof to === 'string' && to.trim() === '*') {
		return parse([from, Infinity]);
	}

	if (
		typeof from === 'string' ||
		typeof to === 'string' ||
		isEqual(input, [0, 0]) ||
		from < 0 ||
		to < 0 ||
		input.length !== ORDERED_LIST_LENGTH ||
		from > to ||
		!isInteger(from) ||
		(!isInteger(to) && to !== Infinity)
	) {
		throw new Error(`Invalid cardinality: ${input}`);
	}

	return {
		from,
		to,
		finite: to !== Infinity,
	};
};
