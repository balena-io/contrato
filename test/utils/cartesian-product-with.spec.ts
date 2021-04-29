/*
 * Copyright 2017 resin.io
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

'use strict'

import test from 'ava';
import * as _ from 'lodash';
import { cartesianProductWith } from '../../lib/utils';

test('should perform a cartesian product of empty sets', (test) => {
	const product = cartesianProductWith([
		[],
		[],
		[]
	], (accumulator, element) => {
		return _.concat(accumulator, [ element ])
	}, [[]])

	test.deepEqual(product, [])
})

test('should perform a cartesian product of a valid and an empty set', (test) => {
	const product = cartesianProductWith([
		[ 'foo' ],
		[],
		[]
	], (accumulator: string[], element: string) => {
		return _.concat(accumulator, [ element ])
	}, [[]])

	test.deepEqual(product, [ [ 'foo' ] ])
})

test('should perform a cartesian product of no sets', (test) => {
	const product = cartesianProductWith([], (accumulator: any[], element: any) => {
		return _.concat(accumulator, [ element ])
	}, [[]])

	test.deepEqual(product, [])
})

test('should perform a cartesian product of a one element set', (test) => {
	const product = cartesianProductWith([
		[ 'foo' ]
	], (accumulator: string[], element: string) => {
		return _.concat(accumulator, [ element ])
	}, [[]])

	test.deepEqual(product, [
		[ 'foo' ]
	])
})

test('should calculate the cartesian product of two string sets', (test) => {
	const product = cartesianProductWith([
		[ 'hello', 'hi', 'hey' ],
		[ 'there', 'world', 'yo' ]
	], (accumulator: string[], element: string) => {
		return _.concat(accumulator, [ element ])
	}, [[]])

	test.deepEqual(product, [
		[ 'hello', 'there' ],
		[ 'hello', 'world' ],
		[ 'hello', 'yo' ],
		[ 'hi', 'there' ],
		[ 'hi', 'world' ],
		[ 'hi', 'yo' ],
		[ 'hey', 'there' ],
		[ 'hey', 'world' ],
		[ 'hey', 'yo' ]
	])
})

test('should be able to discard combinations by returning undefined', (test) => {
	const product = cartesianProductWith([
		[ 'hello', 'hi', 'hey' ],
		[ 'there', 'world', 'yo' ]
	], (accumulator: string[], element: string) => {
		if (!_.isEqual(accumulator, [ 'hello' ]) && element === 'world') {
			return undefined
		}

		if (_.isEqual(accumulator, [ 'hello' ]) && element === 'yo') {
			return undefined
		}

		if (_.isEqual(accumulator, [ 'hello' ]) && element === 'there') {
			return undefined
		}

		return _.concat(accumulator, [ element ])
	}, [[]])

	test.deepEqual(product, [
		[ 'hello', 'world' ],
		[ 'hi', 'there' ],
		[ 'hi', 'yo' ],
		[ 'hey', 'there' ],
		[ 'hey', 'yo' ]
	])
})

test('should be able to discard combinations on a 3 sets product', (test) => {
	const product = cartesianProductWith([
		[ 1, 2, 3 ],
		[ 4, 5, 6 ],
		[ 7, 8, 9 ]
	], (accumulator: number[], element: number) => {
		const combination = _.concat(accumulator, [ element ])

		// Lets pretend we don't want any combination starting
		// with two odd numbers
		if (_.size(combination) > 1 && _.every(combination, (item) => {
			return item % 2 === 1
		})) {
			return undefined
		}

		return combination
	}, [[]])

	test.deepEqual(product, [
		[ 1, 4, 7 ],
		[ 1, 4, 8 ],
		[ 1, 4, 9 ],
		[ 1, 6, 7 ],
		[ 1, 6, 8 ],
		[ 1, 6, 9 ],
		[ 2, 4, 7 ],
		[ 2, 4, 8 ],
		[ 2, 4, 9 ],
		[ 2, 5, 7 ],
		[ 2, 5, 8 ],
		[ 2, 5, 9 ],
		[ 2, 6, 7 ],
		[ 2, 6, 8 ],
		[ 2, 6, 9 ],
		[ 3, 4, 7 ],
		[ 3, 4, 8 ],
		[ 3, 4, 9 ],
		[ 3, 6, 7 ],
		[ 3, 6, 8 ],
		[ 3, 6, 9 ]
	])
})