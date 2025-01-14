/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import concat from 'lodash/concat';
import get from 'lodash/get';
import set from 'lodash/set';
import type Contract from './contract';

/**
 * @summary Get the path of a matcher in the cache
 * @function
 * @private
 *
 * @param {Object} matcher - matcher contract
 * @returns {String[]} path in cache
 *
 * @example
 * const path = getMatcherCachePath(Contract.createMatcher({
 *   type: 'sw.os',
 *   slug: 'debian'
 * }))
 */
const getMatcherCachePath = (matcher: Contract): string[] => [
	matcher.raw.data.type,
	matcher.metadata.hash,
];

/**
 * @ignore
 */
export default class MatcherCache {
	data: any;
	types: Set<string>;
	/**
	 * @summary A data structure to store a cache of matchers
	 * @name MatcherCache
	 * @class
	 * @private
	 *
	 * @example
	 * const cache = new MatcherCache()
	 */
	constructor() {
		this.data = {};
		this.types = new Set();
	}

	/**
	 * @summary Add a value to the matcher cache
	 * @function
	 * @public
	 *
	 * @param {Object} matcher - matcher contract
	 * @param {Any} value - cached value
	 *
	 * @example
	 * const cache = new MatcherCache()
	 * const matcher = Contract.createMatcher({
	 *   type: 'sw.os',
	 *   slug: 'debian'
	 * })
	 *
	 * cache.add(matcher, {
	 *   foo: 'bar'
	 * })
	 */
	add(matcher: Contract, value: any) {
		set(this.data, getMatcherCachePath(matcher), {
			value,
			matcher,
		});

		this.types.add(matcher.raw.data.type);
	}

	/**
	 * @summary Attempt to retrieve a value from the cache
	 * @function
	 * @public
	 *
	 * @param {Object} matcher - matcher contract
	 * @returns {Any} value
	 *
	 * @example
	 * const cache = new MatcherCache()
	 *
	 * const matcher = Contract.createMatcher({
	 *   type: 'sw.os',
	 *   slug: 'debian'
	 * })
	 *
	 * cache.add(matcher, {
	 *   foo: 'bar'
	 * })
	 *
	 * console.log(cache.get(matcher))
	 * > {
	 * >   foo: 'bar'
	 * > }
	 */
	get(matcher: Contract): any {
		const path = getMatcherCachePath(matcher);
		return get(this.data, concat(path, ['value']), null);
	}

	/**
	 * @summary Get the list of types known in the cache
	 * @function
	 * @public
	 *
	 * @returns {Set} types
	 *
	 * @example
	 * const cache = new MatcherCache()
	 *
	 * cache.add(Contract.createMatcher({
	 *   type: 'sw.os',
	 *   slug: 'debian'
	 * }), {
	 *   foo: 'bar'
	 * })
	 *
	 * console.log(cache.getTypes())
	 * > Set ('sw.os')
	 */
	getTypes(): Set<string> {
		return this.types;
	}

	/**
	 * @summary Remove all the cache entries of a specific type
	 * @function
	 * @public
	 *
	 * @param {String} type - contract type
	 *
	 * @example
	 * const cache = new MatcherCache()
	 *
	 * const matcher = Contract.createMatcher({
	 *   type: 'sw.os',
	 *   slug: 'debian'
	 * })
	 *
	 * cache.add(matcher, {
	 *   foo: 'bar'
	 * })
	 *
	 * cache.resetType('sw.os')
	 *
	 * console.log(cache.get(matcher))
	 * > null
	 */
	resetType(type: string) {
		Reflect.deleteProperty(this.data, type);
		this.types.delete(type);
	}

	/**
	 * @summary Merge a cache with another cache
	 * @function
	 * @public
	 *
	 * @param {Object} cache - source cache
	 * @returns {Object} cache instance
	 *
	 * @example
	 * const cache1 = new MatcherCache()
	 * cache1.add({ ... }, [ ... [)
	 * cache1.add({ ... }, [ ... [)
	 *
	 * const cache2 = new MatcherCache()
	 * cache2.add({ ... }, [ ... [)
	 * cache2.add({ ... }, [ ... [)
	 *
	 * cache1.merge(cache2)
	 */
	merge(cache: MatcherCache): this {
		for (const type of cache.getTypes()) {
			if (this.data[type]) {
				this.resetType(type);
			} else {
				this.data[type] = cache.data[type];
			}
		}

		return this;
	}
}
