/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';

import Contract from '../../../lib/contract';
import CONTRACTS from '../../contracts.json';

describe('Contract hash', () => {
	it('should hash the contract by default', () => {
		const contract = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		expect(typeof contract.hash()).to.equal('string');
	});
});
