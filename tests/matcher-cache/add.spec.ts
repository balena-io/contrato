/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import MatcherCache from '../../lib/matcher-cache';
import Contract from '../../lib/contract';

describe('MatcherCache add', () => {
	it('should add one value to the cache', () => {
		const cache = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		cache.add(matcher1, true);

		expect(cache.data).to.deep.equal({
			'sw.os': {
				[matcher1.hash()]: {
					value: true,
					matcher: matcher1,
				},
			},
		});
	});

	it('should add two values to the cache using the same type', () => {
		const cache = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'fedora',
		});

		cache.add(matcher1, true);
		cache.add(matcher2, false);

		expect(cache.data).to.deep.equal({
			'sw.os': {
				[matcher1.hash()]: {
					value: true,
					matcher: matcher1,
				},
				[matcher2.hash()]: {
					value: false,
					matcher: matcher2,
				},
			},
		});
	});

	it('should add two values to the cache using different types', () => {
		const cache = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.stack',
			slug: 'nodejs',
		});

		cache.add(matcher1, true);
		cache.add(matcher2, false);

		expect(cache.data).to.deep.equal({
			'sw.os': {
				[matcher1.hash()]: {
					value: true,
					matcher: matcher1,
				},
			},
			'sw.stack': {
				[matcher2.hash()]: {
					value: false,
					matcher: matcher2,
				},
			},
		});
	});
});
