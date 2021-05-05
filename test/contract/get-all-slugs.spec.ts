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

import Contract from '../../lib/contract';

test('should return only the slug given a contract without aliases', (test) => {
	const contract = new Contract({
		type: 'arch.sw',
		name: 'armv7hf',
		slug: 'armv7hf'
	})

	test.deepEqual(contract.getAllSlugs(), new Set([ 'armv7hf' ]))
})

test('should include the aliases if present', (test) => {
	const contract = new Contract({
		type: 'hw.device-type',
		name: 'Raspberry Pi',
		slug: 'raspberrypi',
		aliases: [ 'rpi', 'raspberry-pi' ]
	})

	test.deepEqual(contract.getAllSlugs(), new Set([
		'rpi',
		'raspberry-pi',
		'raspberrypi'
	]))
})

test('should return only the slug if aliases is empty', (test) => {
	const contract = new Contract({
		type: 'arch.sw',
		name: 'armv7hf',
		slug: 'armv7hf',
		aliases: []
	})

	test.deepEqual(contract.getAllSlugs(), new Set([ 'armv7hf' ]))
})
