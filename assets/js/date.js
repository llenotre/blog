function format_date_long(e) {
    e.innerHTML = dayjs(e.innerHTML).format('HH:mm, dddd MMMM D, YYYY');
}

document.querySelectorAll('[id=date]').forEach(e => {
    e.innerHTML = dayjs(e.innerHTML).format('dddd MMMM D, YYYY');
});
document.querySelectorAll('[id=date-long]').forEach(e => format_date_long(e));
