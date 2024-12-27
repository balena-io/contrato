/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';
import * as _ from 'lodash';

import Contract from '../../../lib/contract';
import Blueprint from '../../../lib/blueprint';
import { hashObject } from '../../../lib/hash';

describe('Blueprint reproduce', () => {
	_.each(
		[
			'1-to-1',
			'1-to-1-skeleton-no-template',
			'1-to-1-skeleton-template',
			'1-to-2',
			'1-to-all',
			'cartesian-simple-2',
			'reference-single',
			'reference-multiple',
			'reference-nested',
			'requirements-or-2',
			'requirements-simple-2',
			'requirements-simple-2-aliases',
		],
		(testName) => {
			// eslint-disable-next-line @typescript-eslint/no-require-imports
			const testCase = require(`./${testName}.json`);

			it(testName, () => {
				const contracts = _.flatMap(testCase.universe, Contract.build);
				const container = new Contract({
					type: 'meta.universe',
				});

				container.addChildren(contracts);

				const blueprint = new Blueprint(
					testCase.blueprint.layout,
					testCase.blueprint.skeleton,
				);
				const result = blueprint.reproduce(container);
				expect(testCase.contexts).to.deep.equal(_.invokeMap(result, 'toJSON'));
			});
		},
	);

	it('should consider the skeleton when computing the hashes', () => {
		const blueprint = new Blueprint(
			{
				'hw.device-type': 1,
			},
			{
				type: 'hw.context.device-type',
				foo: 'bar',
				bar: {
					baz: 1,
				},
			},
		);

		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Intel Edison',
			slug: 'intel-edison',
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Intel NUC',
			slug: 'intel-nuc',
		});

		const container = new Contract({
			type: 'meta.universe',
		});

		container.addChildren([contract1, contract2]);
		const contexts = blueprint.reproduce(container);

		for (const context of contexts) {
			expect(context.metadata.hash).to.equal(hashObject(context.raw));
		}
	});
});
