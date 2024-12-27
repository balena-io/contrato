/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';

import * as _ from 'lodash';

import Contract from '../../../lib/contract';
import Blueprint from '../../../lib/blueprint';

describe('Blueprint sequence', () => {
	_.each(['path_finding'], (testName) => {
		// eslint-disable-next-line @typescript-eslint/no-require-imports
		const testCase = require(`./${testName}.json`);

		it(testName, () => {
			const contracts = _.flatMap(testCase.universe1, Contract.build);
			const container = new Contract({
				type: 'meta.universe',
			});

			container.addChildren(contracts);

			const contracts2 = _.flatMap(testCase.universe2, Contract.build);
			const container2 = new Contract({
				type: 'meta.universe',
			});

			container2.addChildren(contracts2);

			const blueprint = new Blueprint(
				testCase.blueprint1.layout,
				testCase.blueprint1.template,
			);
			const result = blueprint.sequence(container);
			expect(testCase.path1).to.deep.equal(_.invokeMap(result, 'toJSON'));

			const blueprint2 = new Blueprint(
				testCase.blueprint2.layout,
				testCase.blueprint2.template,
			);
			const result2 = blueprint2.sequence(container2);
			expect(testCase.path2).to.deep.equal(_.invokeMap(result2, 'toJSON'));
		});
	});
});
