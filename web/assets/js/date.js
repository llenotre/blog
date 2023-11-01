function format_date(elements) {
    elements.forEach(e => {
        e.innerHTML = dayjs(e.innerHTML).format("dddd MMMM D, YYYY");
    });
}
format_date(document.querySelectorAll("[id=date]"));

function format_date_long(elements) {
    elements.forEach(e => {
        e.innerHTML = dayjs(e.innerHTML).format("HH:mm, dddd MMMM D, YYYY");
    });
}
format_date_long(document.querySelectorAll("[id=date-long]"));
