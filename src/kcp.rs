/// all time value is milliseconds
/// retransmission timeout with no delay but at least 30 ms
const RTO_NDL: u64 = 30;
/// the min value of retransmission timeout
const RTO_MIN: u64 = 100;
/// the normal value of retransmission timeout
const RTO_DEF: u64 = 200;
/// the max value of retransmission timeout
const RTO_MAX: u64 = 60000;
