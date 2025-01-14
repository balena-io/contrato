/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import get from 'lodash/get';
import isArray from 'lodash/isArray';
import isPlainObject from 'lodash/isPlainObject';
import isString from 'lodash/isString';
import map from 'lodash/map';
import mapValues from 'lodash/mapValues';

import type { ContractObject } from './types';

/**
 * @module template
 */

/**
 * @summary Map object values recursively
 * @function
 * @private
 *
 * @param {Object} object - object
 * @param {Function} callback - callback (value)
 * @param {String[]} [breadcrumb] - location breadcrumb
 * @returns {Object} mapped object
 *
 * @example
 * const result = deepMapValues({
 *   a: 1,
 *   b: 2,
 *   c: 3
 * }, (value) => {
 *   return value * 2
 * })
 *
 * console.log(result)
 * > {
 * >   a: 2
 * >   b: 4
 * >   c: 6
 * > }
 */
const deepMapValues = (
	object: object,
	callback: (arg0: object, arg1: string[]) => object,
	breadcrumb: string[] = [],
): object =>
	!isPlainObject(object)
		? callback(object, breadcrumb)
		: mapValues(object, (value: any, key) => {
				const absoluteKey = [...breadcrumb, key];
				return isPlainObject(value)
					? deepMapValues(value, callback, absoluteKey)
					: callback(value, absoluteKey);
			});

/**
 * @summary Contract template interpolation regex
 * @type {RegExp}
 * @constant
 */
const TEMPLATE_REGEXP = /\{\{(.+?)\}\}/g;
/**
 * @summary Compile contract templates
 * @function
 * @public
 * @memberof module:template
 *
 * @param {Object} contract - contract
 * @param {Object} [options] - options
 * @param {Set} [options.blacklist] - paths to ignore
 * @param {Object} [root] - template root (internally used during recursion)
 * @param {String[]} [breadcrumb] - breadcrumb (internally used during recursion)
 * @returns {Object} applied contract
 *
 * @example
 * const contract = template.compileContract({
 *   type: 'distro',
 *   name: 'debian {{this.version}}',
 *   version: 'wheezy',
 *   slug: 'debian'
 * })
 *
 * console.log(contract)
 * > {
 * >   type: 'distro',
 * >   name: 'debian wheezy',
 * >   version: 'wheezy',
 * >   slug: 'debian'
 * > }
 */
export const compileContract = (
	contract: ContractObject,
	options: { blacklist?: Set<string> } = {},
	root = contract,
	breadcrumb?: string[],
): ContractObject => {
	let isMultiLevelBlacklist = false;
	if (options.blacklist) {
		for (const path of options.blacklist) {
			if (path.includes('.')) {
				isMultiLevelBlacklist = true;
				break;
			}
		}
	}
	return deepMapValues(
		contract,
		(value: any, key: string[]) => {
			if (isString(value)) {
				if (options.blacklist) {
					const checkKey = isMultiLevelBlacklist ? key.join('.') : key[0];
					for (const path of options.blacklist) {
						if (checkKey.startsWith(path)) {
							return value;
						}
					}
				}
				return value.replace(
					TEMPLATE_REGEXP,
					(interpolation: string, path: string) => {
						const pathArr = path.split('.');
						if (pathArr.shift() !== 'this') {
							// Ensure we started with `this` as it's the only path we allow
							// and checking in this way avoids creating an unnecessary object
							return interpolation;
						}
						return get(root, pathArr) || interpolation;
					},
				);
			}
			if (isArray(value)) {
				return map(value as ContractObject[], (object, index) =>
					compileContract(object, options, contract, [
						...key,
						index.toString(),
					]),
				);
			}
			return value;
		},
		breadcrumb,
	) as ContractObject;
};
