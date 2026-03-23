/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import isEmpty from 'lodash/isEmpty';
import clone from 'lodash/clone';

/**
 * @module utils
 */

/**
 * @summary Check if two sets are disjoint
 * @function
 * @memberof module:utils
 * @public
 *
 * @param {Set} set1 - first set
 * @param {Set} set2 - second set
 * @returns {Boolean} whether the sets are disjoint
 *
 * @example
 * const set1 = new Set([ 'foo', 'bar' ])
 * const set2 = new Set([ 'baz', 'qux' ])
 *
 * if (utils.areSetsDisjoint(set1, set2)) {
 *   console.log('These sets are disjoint')
 * }
 */
export const areSetsDisjoint = <T>(set1: Set<T>, set2: Set<T>): boolean => {
	for (const element of set1) {
		if (set2.has(element)) {
			return false;
		}
	}

	return true;
};

/**
 * @summary Perform an union operation between sets
 * @function
 * @memberof module:utils
 * @public
 *
 * @param {Set} set1 - first set
 * @param {Set} set2 - second set
 * @returns {Set} union result
 *
 * @example
 * const set1 = new Set([ 1, 2 ])
 * const set2 = new Set([ 3, 4 ])
 *
 * console.log(utils.setUnion(set1, set2))
 * > Set { 1, 2, 3, 4 }
 */
export const setUnion = <T>(set1: Set<T>, set2: Set<T>): Set<T> => {
	const union = new Set(set1);

	for (const element of set2) {
		union.add(element);
	}

	return union;
};

/**
 * @summary Get the first element of a set
 * @function
 * @memberof module:utils
 * @public
 *
 * @param {Set} set1 - set
 * @returns {*} the first set element
 *
 * @example
 * const set = new Set([ 'foo' ])
 * const element = utils.setFirst(set)
 * console.log(element)
 * > 'foo'
 */
export const setFirst = <T>(set1: Set<T>): T | undefined =>
	set1.values().next().value;

/**
 * @summary Map a set using an iteratee function
 * @function
 * @memberof module:utils
 * @public
 *
 * @param {Set} set1 - set
 * @param {Function} iteratee - iteratee (value)
 * @returns {Array} mapped set
 *
 * @example
 * const result = utils.setMap(new Set([ 1, 2, 3 ]), (value) => {
 *   return value * 2
 * })
 *
 * console.log(result)
 * > [ 2, 4, 6 ]
 */
export const setMap = <T, V>(set1: Set<T>, iteratee: (arg0: T) => V): V[] => {
	const result: V[] = [];

	for (const element of set1) {
		result.push(iteratee(element));
	}

	return result;
};

/**
 * @summary Perform a depth 1 flatten operation on an iterable
 * @function
 * @memberof module:utils
 * @public
 *
 * @description
 * This function performs the equivalent of Array.flat but for an iterable input.
 * Given an iterable of arrays, this function will return an iterator over the elements of the arrays.
 *
 * @param {Iterable} iterable - iterator
 * @returns {Iterable} mapped iterator
 */
export function* flatten<T>(iterable: Iterable<T | T[]>): IterableIterator<T> {
	for (const it of iterable) {
		if (!Array.isArray(it)) {
			yield it;
			continue;
		}

		for (const item of it) {
			yield item;
		}
	}
}

/**
 * @summary Perform a filter operation on an iterable input
 * @function
 * @memberof module:utils
 * @public
 *
 * @description
 * This function performs the equivalent of Array.filter() but for an iterable input.
 * Given an iterable and a predicate function, this function will return an iterator
 * over the elements of the iterable that satisfy the predicate.
 *
 * @param {Iterable} iterable - iterator
 * @param {Function} predicate - predicate function
 * @returns {Iterable} filtered iterator
 */
export function* filter<T>(
	iterable: Iterable<T>,
	predicate: (t: T) => boolean,
): IterableIterator<T> {
	for (const it of iterable) {
		if (predicate(it)) {
			yield it;
		}
	}
}

function* nextCartesianProduct<T, V>(
	sets: T[][],
	iteratee: (arg0: V, arg1: T) => V | undefined,
	combination: V,
	setRow: number,
	setCol: number,
): IterableIterator<V> {
	// If we have reached the last row of the sets, then we have a complete combination
	// and we can yield a result if the combination is not empty
	if (setRow >= sets.length) {
		if (!isEmpty(combination)) {
			yield combination;
		}
		return;
	}

	const set = sets[setRow];

	// If the row is empty, then we skip it and go to the next row
	if (set.length === 0) {
		yield* nextCartesianProduct(sets, iteratee, combination, setRow + 1, 0);
		return;
	}

	// Calculate first the combination with the values up to the current row
	const value = set[setCol];
	const newCombination = iteratee(clone(combination), value);
	if (newCombination) {
		yield* nextCartesianProduct(sets, iteratee, newCombination, setRow + 1, 0);
	}

	// Then iterate the columns
	if (setCol + 1 < set.length) {
		yield* nextCartesianProduct(
			sets,
			iteratee,
			combination,
			setRow,
			setCol + 1,
		);
	}
}

/**
 * @summary Compute the cartisian product of a set of sets and return an iterable over the results
 * @function
 * @memberof module:utils
 * @public
 *
 * @description
 * This function combines the first two sets, and then combines
 * the resulting set if the third set, and so on.
 *
 * The iteratee function is called every step, and clients may
 * return undefined to stop evaluating a possible combination.
 *
 * This function performs the calculation in a depth first way, to avoid keeping the full list
 * of values in memory, and instead it yields the results as they are calculated.
 *
 * @param {Array[]} sets - sets of sets
 * @param {Function} iteratee - iteratee (accumulator, element)
 * @returns {Iterable} cartesian product iterator
 *
 * @example
 * const product = utils.iterableCartesianProductWith([
 *   [ 1, 2 ],
 *   [ 3, 4 ]
 * ], (accumulator, element) => {
 *   return accumulator.concat([ element ])
 * })
 *
 * for (const combination of product) {
 *  console.log(combination)
 * }
 *
 * > [ 1, 3 ],
 * > [ 1, 4 ],
 * > [ 2, 3 ],
 * > [ 2, 4 ]
 */

export function* cartesianProductWith<T, V>(
	sets: T[][],
	iteratee: (arg0: V, arg1: T) => V | undefined,
	init: V[],
): IterableIterator<V> {
	for (const combination of init) {
		yield* nextCartesianProduct(sets, iteratee, combination, 0, 0);
	}
}

/**
 * @summary Strip extra blank lines from a multi-line text
 * @function
 * @memberof module:utils
 * @public
 *
 * @param {String} text - text
 * @returns {String} text without the extra blank lines
 *
 * @example
 * const stripped = utils.stripExtraBlankLines('...')
 */
export const stripExtraBlankLines = (text: string): string =>
	text.replace(/(\r?\n){3,}/g, '\n\n').trim();
