/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';

import Contract from '../../../lib/contract';
import CONTRACTS from '../../contracts.json';

describe('Contract hash', () => {
	it('should compute the contract hash lazily', () => {
		const contract = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		// The hash is not computed until requested
		expect(typeof contract.metadata.hash).to.equal('undefined');

		expect(typeof contract.hash()).to.equal('string');

		// Once computed, it is cached
		expect(typeof contract.metadata.hash).to.equal('string');
	});
});
