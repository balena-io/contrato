/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import MatcherCache from '../../lib/matcher-cache';
import Contract from '../../lib/contract';

describe('MatcherCache merge', () => {
	it('should merge two empty caches', () => {
		const cache1 = new MatcherCache();
		const cache2 = new MatcherCache();
		cache1.merge(cache2);
		expect(cache1.data).to.deep.equal({});
	});

	it('should merge a non empty cache with one type into an empty cache', () => {
		const cache1 = new MatcherCache();
		const cache2 = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'fedora',
		});

		cache2.add(matcher1, true);
		cache2.add(matcher2, true);

		cache1.merge(cache2);

		expect(cache1.data).to.deep.equal({
			'sw.os': {
				[matcher1.hash()]: {
					value: true,
					matcher: matcher1,
				},
				[matcher2.hash()]: {
					value: true,
					matcher: matcher2,
				},
			},
		});
	});

	it('should merge a non empty cache with two types into an empty cache', () => {
		const cache1 = new MatcherCache();
		const cache2 = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.blob',
			slug: 'nodejs',
		});

		cache2.add(matcher1, true);
		cache2.add(matcher2, true);

		cache1.merge(cache2);

		expect(cache1.data).to.deep.equal({
			'sw.os': {
				[matcher1.hash()]: {
					value: true,
					matcher: matcher1,
				},
			},
			'sw.blob': {
				[matcher2.hash()]: {
					value: true,
					matcher: matcher2,
				},
			},
		});
	});

	it('should merge two non empty caches with disjoint types', () => {
		const cache1 = new MatcherCache();
		const cache2 = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.blob',
			slug: 'nodejs',
		});

		cache1.add(matcher1, true);
		cache2.add(matcher2, true);

		cache1.merge(cache2);

		expect(cache1.data).to.deep.equal({
			'sw.os': {
				[matcher1.hash()]: {
					value: true,
					matcher: matcher1,
				},
			},
			'sw.blob': {
				[matcher2.hash()]: {
					value: true,
					matcher: matcher2,
				},
			},
		});
	});

	it('should omit types in common', () => {
		const cache1 = new MatcherCache();
		const cache2 = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.blob',
			slug: 'nodejs',
		});

		const matcher3 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'fedora',
		});

		cache1.add(matcher1, true);
		cache2.add(matcher2, true);
		cache2.add(matcher3, true);

		cache1.merge(cache2);

		expect(cache1.data).to.deep.equal({
			'sw.blob': {
				[matcher2.hash()]: {
					value: true,
					matcher: matcher2,
				},
			},
		});
	});

	it('should return the instance', () => {
		const cache1 = new MatcherCache();
		const cache2 = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.blob',
			slug: 'nodejs',
		});

		const matcher3 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'fedora',
		});

		cache1.add(matcher1, true);
		cache2.add(matcher2, true);
		cache2.add(matcher3, true);

		expect(cache1.merge(cache2)).to.deep.equal(cache1);
	});
});
