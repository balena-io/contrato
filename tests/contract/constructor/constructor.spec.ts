/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';

import MatcherCache from '../../../lib/matcher-cache';
import Contract from '../../../lib/contract';
import ObjectSet from '../../../lib/object-set';

describe('Contract constructor', () => {
	it('should create a simple contract', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		expect(contract.metadata).to.deep.equal({
			children: {
				typeMatchers: {},
				searchCache: new MatcherCache(),
				types: new Set(),
				map: {},
				byType: {},
				byTypeSlug: {},
			},
			requirements: {
				matchers: {},
				types: new Set(),
				compiled: new ObjectSet(),
			},
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});
	});
});
