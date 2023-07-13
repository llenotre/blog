document.querySelectorAll('[id=date]').forEach(e => {
    e.innerHTML = dayjs(e.innerHTML).format('dddd MMMM D, YYYY');
});
document.querySelectorAll('[id=date-long]').forEach(e => {
    e.innerHTML = dayjs(e.innerHTML).format('HH:mm, dddd MMMM D, YYYY');
});
