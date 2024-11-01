// Get the button and dropdown content
const dropbtn = document.querySelector('.dropbtn');
const dropdownContent = document.getElementById('myDropdown');

// Toggle the dropdown content on button click
dropbtn.addEventListener('click', function() {
    dropdownContent.classList.toggle('show');
});

// Close the dropdown if clicked outside of it
window.addEventListener('click', function(event) {
    if (!event.target.matches('.dropbtn')) {
        if (dropdownContent.classList.contains('show')) {
            dropdownContent.classList.remove('show');
        }
    }
});
