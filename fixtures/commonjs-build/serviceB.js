"use strict";
// CommonJS cycle: serviceB -> serviceA -> serviceB
const serviceA = require('./serviceA');

function helper() {
  return 'helped';
}

function callA() {
  return serviceA.doSomething();
}

module.exports = { helper, callA };
