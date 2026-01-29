"use strict";
// Entry point - no cycle
const serviceA = require('./serviceA');
const utils = require('./utils');

module.exports = { serviceA, utils };
