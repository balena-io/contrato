/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import MatcherCache from '../../lib/matcher-cache';
import Contract from '../../lib/contract';

describe('MatcherCache resetType', () => {
	it('should remove all specified types from the cache', () => {
		const cache = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'fedora',
		});

		const matcher3 = Contract.createMatcher({
			type: 'sw.stack',
			slug: 'nodejs',
		});

		cache.add(matcher1, true);
		cache.add(matcher2, false);
		cache.add(matcher3, true);

		cache.resetType('sw.os');

		expect(cache.data).to.deep.equal({
			'sw.stack': {
				[matcher3.hash()]: {
					value: true,
					matcher: matcher3,
				},
			},
		});
	});

	it('should do nothing if the type does not exist', () => {
		const cache = new MatcherCache();

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'fedora',
		});

		const matcher3 = Contract.createMatcher({
			type: 'sw.stack',
			slug: 'nodejs',
		});

		cache.add(matcher1, true);
		cache.add(matcher2, false);
		cache.add(matcher3, true);

		cache.resetType('foobar');

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
			'sw.stack': {
				[matcher3.hash()]: {
					value: true,
					matcher: matcher3,
				},
			},
		});
	});
});
