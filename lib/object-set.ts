/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { hashObject } from './hash';

/**
 * @ignore
 */
export default class ObjectSet {
	cache: { size: number };
	data: Record<string, object>;
	/**
	 * @summary A data structure to represent a set of objects
	 * @name ObjectSet
	 * @class
	 * @private
	 *
	 * @param {(Object[]|Array[])} [objects] - initial objects
	 *
	 * @example
	 * const set = new ObjectSet([
	 *   { ... },
	 *   { ... },
	 *   { ... }
	 * ])
	 */
	constructor(objects: Array<object | object[]> = []) {
		this.cache = {
			size: 0,
		};
		this.data = {};
		for (const object of objects) {
			if (Array.isArray(object)) {
				const firstObj = object[0];
				const lastObj = object[object.length - 1];
				if (firstObj && lastObj) {
					this.add(firstObj, lastObj);
				}
			} else {
				this.add(object);
			}
		}
	}
	/**
	 * @summary Add an object to the set
	 * @function
	 * @public
	 *
	 * @param {Object} object - object
	 * @param {Object} [options] - options
	 * @param {String} [options.id] - custom unique id for the object
	 *
	 * @example
	 * const set = new ObjectSet()
	 * set.add({ foo: 'bar' })
	 * set.add({ foo: 'baz' }, { id: 'myuniqueid' })
	 */
	add(object: object, options: object = {}) {
		// eslint-disable-next-line @typescript-eslint/prefer-nullish-coalescing
		const id = (options as any).id || hashObject(object);
		if (this.hasId(id)) {
			return;
		}
		this.data[id] = object;
		this.cache.size++;
	}
	/**
	 * @summary Get the current size of the set
	 * @function
	 * @public
	 *
	 * @returns {Number} size
	 *
	 * @example
	 * const set = new ObjectSet()
	 *
	 * if (set.size() === 0) {
	 *   console.log('This set is empty')
	 * }
	 */
	size(): number {
		return this.cache.size;
	}
	/**
	 * @summary Check if the set has a certain object
	 * @function
	 * @public
	 *
	 * @param {Object} object - object
	 * @returns {Boolean} whether the set has the object
	 *
	 * @example
	 * const set = new ObjectSet([
	 *   { foo: 'bar' }
	 * ])
	 *
	 * if (set.hasObject({ foo: 'bar' })) {
	 *   console.log('This object is there!')
	 * }
	 */
	hasObject(object: object): boolean {
		return Boolean(this.data[hashObject(object)]);
	}
	/**
	 * @summary Check if the set has a certain id
	 * @function
	 * @public
	 *
	 * @param {String} id - id
	 * @returns {Boolean} whether the set has the id
	 *
	 * @example
	 * const set = new ObjectSet()
	 * set.add({ foo: 'bar' }, { id: 'foo' })
	 *
	 * if (set.hasId('foo')) {
	 *   console.log('This id is there!')
	 * }
	 */
	hasId(id: string): boolean {
		return Boolean(this.data[id]);
	}
	/**
	 * @summary Get all the objects in the set
	 * @function
	 * @public
	 *
	 * @returns {Object[]} all the objects in the set
	 *
	 * @example
	 * const set = new ObjectSet([
	 *   { ... },
	 *   { ... },
	 *   { ... }
	 * ])
	 *
	 * set.getAll().forEach((object) => {
	 *   console.log(object)
	 * })
	 */
	getAll(): object[] {
		return Object.values(this.data);
	}
	/**
	 * @summary Calculate the intersection between two object sets
	 * @function
	 * @public
	 *
	 * @param {Object} set - object set
	 * @returns {Object} intersection set
	 *
	 * @example
	 * const set1 = new ObjectSet([
	 *   { foo: 'bar' }
	 *   { bar: 'baz' }
	 * ])
	 *
	 * const set2 = new ObjectSet([
	 *   { bar: 'baz' }
	 * ])
	 *
	 * set1.intersection(set2)
	 *
	 * console.log(set1.getAll())
	 * > [
	 * >   { bar: 'baz' }
	 * > ]
	 */
	intersection(set: ObjectSet): object {
		for (const id of Object.keys(this.data)) {
			if (!set.hasId(id)) {
				Reflect.deleteProperty(this.data, id);
			}
		}
		return this;
	}
}
